use crate::{Config, julia};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Package {
    id: String,
    name: String,
    version: Option<String>,
    direct: bool,
    dependencies: Vec<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Graph {
    packages: BTreeMap<String, Package>,
}

pub fn tree(config: &Config, project: &Path) -> Result<(), String> {
    let Some(output) = julia::dependency_graph(config, project)? else {
        return Ok(());
    };
    let graph = Graph::parse(&output)?;
    print!("{}", graph.render_tree());
    Ok(())
}

pub fn why(config: &Config, project: &Path, package: &str) -> Result<(), String> {
    let Some(output) = julia::dependency_graph(config, project)? else {
        return Ok(());
    };
    let graph = Graph::parse(&output)?;
    print!("{}", graph.explain(package)?);
    Ok(())
}

impl Graph {
    fn parse(input: &str) -> Result<Self, String> {
        let mut packages = BTreeMap::new();
        let mut edges = Vec::new();

        for (index, line) in input.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let fields: Vec<_> = line.split('\t').collect();
            match fields.as_slice() {
                ["node", id, name, version, direct] => {
                    let direct = direct.parse::<bool>().map_err(|_| {
                        format!("invalid direct-dependency flag on graph line {}", index + 1)
                    })?;
                    packages.insert(
                        (*id).to_owned(),
                        Package {
                            id: (*id).to_owned(),
                            name: (*name).to_owned(),
                            version: (!version.is_empty()).then(|| (*version).to_owned()),
                            direct,
                            dependencies: Vec::new(),
                        },
                    );
                }
                ["edge", from, to] => edges.push(((*from).to_owned(), (*to).to_owned())),
                _ => {
                    return Err(format!(
                        "invalid dependency graph data on line {}",
                        index + 1
                    ));
                }
            }
        }

        for (from, to) in edges {
            if !packages.contains_key(&to) {
                continue;
            }
            let package = packages
                .get_mut(&from)
                .ok_or_else(|| format!("dependency graph references unknown package '{from}'"))?;
            package.dependencies.push(to);
        }

        let names: HashMap<_, _> = packages
            .iter()
            .map(|(id, package)| (id.clone(), package.name.clone()))
            .collect();
        for package in packages.values_mut() {
            package.dependencies.sort_by(|left, right| {
                let left = &names[left];
                let right = &names[right];
                left.cmp(right)
            });
        }

        Ok(Self { packages })
    }

    fn render_tree(&self) -> String {
        let mut roots: Vec<_> = self
            .packages
            .values()
            .filter(|package| package.direct)
            .collect();
        roots.sort_by(|left, right| left.name.cmp(&right.name));

        if roots.is_empty() {
            return "No project dependencies.\n".to_owned();
        }

        let mut output = String::new();
        for (index, root) in roots.iter().enumerate() {
            if index > 0 {
                output.push('\n');
            }
            let mut visited = HashSet::new();
            self.render_package(&root.id, "", true, true, &mut visited, &mut output);
        }
        output
    }

    fn render_package(
        &self,
        id: &str,
        prefix: &str,
        last: bool,
        root: bool,
        visited: &mut HashSet<String>,
        output: &mut String,
    ) {
        let package = &self.packages[id];
        if !root {
            output.push_str(prefix);
            output.push_str(if last { "`-- " } else { "|-- " });
        }
        output.push_str(&package.label());

        if !visited.insert(id.to_owned()) {
            output.push_str(" (*)\n");
            return;
        }
        output.push('\n');

        let child_prefix = if root {
            String::new()
        } else {
            format!("{prefix}{}", if last { "    " } else { "|   " })
        };
        for (index, dependency) in package.dependencies.iter().enumerate() {
            self.render_package(
                dependency,
                &child_prefix,
                index + 1 == package.dependencies.len(),
                false,
                visited,
                output,
            );
        }
    }

    fn explain(&self, requested: &str) -> Result<String, String> {
        let target = self.find_by_name(requested)?;
        let path = self.shortest_path(&target.id).ok_or_else(|| {
            format!(
                "{} is installed but not reachable from this project",
                target.name
            )
        })?;

        if path.len() == 1 {
            return Ok(format!(
                "{} is a direct dependency of this project.\n",
                target.label()
            ));
        }

        let mut output = format!("{} is installed because:\n\nProject\n", target.label());
        for (index, id) in path.iter().enumerate() {
            output.push_str(&"    ".repeat(index));
            output.push_str("`-- ");
            output.push_str(&self.packages[id].label());
            output.push('\n');
        }
        Ok(output)
    }

    fn find_by_name(&self, requested: &str) -> Result<&Package, String> {
        if let Some(package) = self
            .packages
            .values()
            .find(|package| package.name == requested)
        {
            return Ok(package);
        }

        let matches: Vec<_> = self
            .packages
            .values()
            .filter(|package| package.name.eq_ignore_ascii_case(requested))
            .collect();
        match matches.as_slice() {
            [package] => Ok(package),
            [] => Err(format!("package '{requested}' is not installed")),
            _ => Err(format!("package name '{requested}' is ambiguous")),
        }
    }

    fn shortest_path(&self, target: &str) -> Option<Vec<String>> {
        let mut roots: Vec<_> = self
            .packages
            .values()
            .filter(|package| package.direct)
            .map(|package| package.id.clone())
            .collect();
        roots.sort_by(|left, right| self.packages[left].name.cmp(&self.packages[right].name));

        let mut queue = VecDeque::new();
        let mut parent: HashMap<String, Option<String>> = HashMap::new();
        for root in roots {
            parent.insert(root.clone(), None);
            queue.push_back(root);
        }

        while let Some(current) = queue.pop_front() {
            if current == target {
                let mut path = Vec::new();
                let mut cursor = Some(current);
                while let Some(id) = cursor {
                    cursor = parent[&id].clone();
                    path.push(id);
                }
                path.reverse();
                return Some(path);
            }

            for dependency in &self.packages[&current].dependencies {
                if !parent.contains_key(dependency) {
                    parent.insert(dependency.clone(), Some(current.clone()));
                    queue.push_back(dependency.clone());
                }
            }
        }
        None
    }
}

impl Package {
    fn label(&self) -> String {
        self.version.as_ref().map_or_else(
            || self.name.clone(),
            |version| format!("{} v{version}", self.name),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRAPH: &str = "\
node\ta\tApp\t1.0.0\ttrue
node\tb\tBridge\t2.0.0\tfalse
node\tc\tCore\t3.0.0\tfalse
node\td\tDirect\t4.0.0\ttrue
edge\ta\tb
edge\tb\tc
edge\td\tc
";

    #[test]
    fn parses_and_renders_dependency_tree() {
        let graph = Graph::parse(GRAPH).unwrap();
        assert_eq!(
            graph.render_tree(),
            "App v1.0.0\n`-- Bridge v2.0.0\n    `-- Core v3.0.0\n\nDirect v4.0.0\n`-- Core v3.0.0\n"
        );
    }

    #[test]
    fn explains_direct_and_transitive_dependencies() {
        let graph = Graph::parse(GRAPH).unwrap();
        assert_eq!(
            graph.explain("App").unwrap(),
            "App v1.0.0 is a direct dependency of this project.\n"
        );
        assert_eq!(
            graph.explain("Core").unwrap(),
            "Core v3.0.0 is installed because:\n\nProject\n`-- Direct v4.0.0\n    `-- Core v3.0.0\n"
        );
    }

    #[test]
    fn reports_unknown_package() {
        let graph = Graph::parse(GRAPH).unwrap();
        assert_eq!(
            graph.explain("Missing").unwrap_err(),
            "package 'Missing' is not installed"
        );
    }
}
