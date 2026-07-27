use crate::{Command, Config};
use std::ffi::{OsStr, OsString};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

#[derive(Debug, PartialEq, Eq)]
pub struct Probe {
    pub version: String,
    pub active_project: String,
    pub depot: String,
    pub registries: usize,
}

pub fn execute(config: &Config, command: &Command, project: &Path) -> Result<(), String> {
    let (args, description) = invocation(command, project)?;

    if config.dry_run {
        print_command(&config.julia, &args);
        return Ok(());
    }

    run_status(&config.julia, &args, description)
}

pub fn generate(config: &Config, path: &Path) -> Result<(), String> {
    let path_text = path
        .to_str()
        .ok_or_else(|| "package paths must be valid UTF-8".to_owned())?;
    let path = absolute_path(path_text)?;
    let name = path
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| format!("cannot infer a package name from '{}'", path.display()))?;

    if !valid_package_name(name) {
        return Err(format!(
            "'{name}' is not a valid Julia package name; use letters, numbers, and underscores"
        ));
    }
    if path.exists() {
        return Err(format!("'{}' already exists", path.display()));
    }

    let expression = format!(
        "import Pkg; Pkg.generate({})",
        julia_string(&path.to_string_lossy())
    );
    let args = base_args(None, expression);

    if config.dry_run {
        print_command(&config.julia, &args);
        return Ok(());
    }

    run_status(&config.julia, &args, "generating the package")?;
    add_test_scaffold(&path, name)?;
    println!("Created Julia package {name} at {}", path.display());
    Ok(())
}

pub fn probe(config: &Config, project: Option<&Path>) -> Result<Option<Probe>, String> {
    let expression = "\
import Pkg
println(\"version\\t\", VERSION)
println(\"project\\t\", something(Base.active_project(), \"none\"))
println(\"depot\\t\", first(DEPOT_PATH))
println(\"registries\\t\", length(Pkg.Registry.reachable_registries()))
";
    let args = base_args(project, expression.to_owned());

    if config.dry_run {
        print_command(&config.julia, &args);
        return Ok(None);
    }

    let output = ProcessCommand::new(&config.julia)
        .args(&args)
        .output()
        .map_err(|error| process_start_error(&config.julia, error))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Julia environment probe failed{}{}",
            output
                .status
                .code()
                .map(|code| format!(" (exit code {code})"))
                .unwrap_or_default(),
            if detail.trim().is_empty() {
                String::new()
            } else {
                format!(": {}", detail.trim())
            }
        ));
    }

    parse_probe(&String::from_utf8_lossy(&output.stdout)).map(Some)
}

pub fn dependency_graph(config: &Config, project: &Path) -> Result<Option<String>, String> {
    let expression = "\
import Pkg
packages = Pkg.dependencies()
for (uuid, info) in sort!(collect(packages); by = pair -> string(pair.first))
    version = something(info.version, \"\")
    println(\"node\\t\", uuid, \"\\t\", info.name, \"\\t\", version, \"\\t\", info.is_direct_dep)
    for dependency in sort!(collect(values(info.dependencies)); by = string)
        haskey(packages, dependency) && println(\"edge\\t\", uuid, \"\\t\", dependency)
    end
end
";
    capture(config, project, expression)
}

pub fn format(config: &Config, project: &Path, paths: &[PathBuf]) -> Result<(), String> {
    let paths = if paths.is_empty() {
        vec![project.to_path_buf()]
    } else {
        paths
            .iter()
            .map(|path| {
                if path.is_absolute() {
                    path.clone()
                } else {
                    std::env::current_dir()
                        .map(|cwd| cwd.join(path))
                        .map_err(|error| format!("cannot resolve format path: {error}"))?
                }
                .canonicalize()
                .map_err(|error| format!("cannot format '{}': {error}", path.display()))
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    let paths = paths
        .iter()
        .map(|path| julia_string(&path.to_string_lossy()))
        .collect::<Vec<_>>()
        .join(", ");
    let expression = format!(
        "\
import Pkg
if Base.find_package(\"JuliaFormatter\") === nothing
    println(stderr, \"Installing JuliaFormatter in the shared jpm tools environment...\")
    Pkg.add(\"JuliaFormatter\")
end
using JuliaFormatter
paths = [{paths}]
JuliaFormatter.format(paths; throw_on_error=true)
"
    );
    let args = vec![
        OsString::from("--project=@jpm-tools"),
        OsString::from("--startup-file=no"),
        OsString::from("--history-file=no"),
        OsString::from("--color=no"),
        OsString::from("-e"),
        OsString::from(expression),
    ];

    if config.dry_run {
        print_command(&config.julia, &args);
        return Ok(());
    }

    run_status(&config.julia, &args, "formatting Julia source")
}

fn invocation(command: &Command, project: &Path) -> Result<(Vec<OsString>, &'static str), String> {
    let project_arg = format!("--project={}", project.display());

    if let Command::Run { args } = command {
        let mut julia_args = vec![OsString::from(project_arg)];
        julia_args.extend(args.iter().cloned());
        return Ok((julia_args, "running the project"));
    }

    let expression = pkg_expression(command)?;
    Ok((
        base_args(Some(project), expression),
        command_description(command),
    ))
}

fn base_args(project: Option<&Path>, expression: String) -> Vec<OsString> {
    let mut args = Vec::with_capacity(6);
    if let Some(project) = project {
        args.push(OsString::from(format!("--project={}", project.display())));
    }
    args.extend([
        OsString::from("--startup-file=no"),
        OsString::from("--history-file=no"),
        OsString::from("--color=no"),
        OsString::from("-e"),
        OsString::from(expression),
    ]);
    args
}

fn run_status(program: &OsStr, args: &[OsString], description: &str) -> Result<(), String> {
    let status = ProcessCommand::new(program)
        .args(args)
        .status()
        .map_err(|error| process_start_error(program, error))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "Julia failed while {description}{}",
            status
                .code()
                .map(|code| format!(" (exit code {code})"))
                .unwrap_or_default()
        ))
    }
}

fn capture(config: &Config, project: &Path, expression: &str) -> Result<Option<String>, String> {
    let args = base_args(Some(project), expression.to_owned());
    if config.dry_run {
        print_command(&config.julia, &args);
        return Ok(None);
    }

    let output = ProcessCommand::new(&config.julia)
        .args(&args)
        .output()
        .map_err(|error| process_start_error(&config.julia, error))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Julia failed while reading the dependency graph{}{}",
            output
                .status
                .code()
                .map(|code| format!(" (exit code {code})"))
                .unwrap_or_default(),
            if detail.trim().is_empty() {
                String::new()
            } else {
                format!(": {}", detail.trim())
            }
        ));
    }

    String::from_utf8(output.stdout)
        .map(Some)
        .map_err(|_| "Julia returned non-UTF-8 dependency data".to_owned())
}

fn process_start_error(program: &OsStr, error: std::io::Error) -> String {
    if error.kind() == std::io::ErrorKind::NotFound {
        format!(
            "Julia executable '{}' was not found; install Julia or set JPM_JULIA",
            program.to_string_lossy()
        )
    } else {
        format!("could not start Julia: {error}")
    }
}

fn add_test_scaffold(path: &Path, name: &str) -> Result<(), String> {
    let test_directory = path.join("test");
    fs::create_dir(&test_directory)
        .map_err(|error| format!("cannot create '{}': {error}", test_directory.display()))?;
    fs::write(
        test_directory.join("runtests.jl"),
        format!(
            "\
using Test
using {name}

@testset \"{name}.jl\" begin
    # Add package tests here.
end
"
        ),
    )
    .map_err(|error| format!("cannot create package tests: {error}"))?;

    let project_file = path.join("Project.toml");
    let mut project = OpenOptions::new()
        .append(true)
        .open(&project_file)
        .map_err(|error| format!("cannot update '{}': {error}", project_file.display()))?;
    project
        .write_all(
            b"\n[extras]\nTest = \"8dfed614-e22c-5e08-85e1-65c5234f0b40\"\n\n[targets]\ntest = [\"Test\"]\n",
        )
        .map_err(|error| format!("cannot update '{}': {error}", project_file.display()))
}

fn parse_probe(output: &str) -> Result<Probe, String> {
    let mut version = None;
    let mut active_project = None;
    let mut depot = None;
    let mut registries = None;

    for line in output.lines() {
        let Some((key, value)) = line.split_once('\t') else {
            continue;
        };
        match key {
            "version" => version = Some(value.to_owned()),
            "project" => active_project = Some(value.to_owned()),
            "depot" => depot = Some(value.to_owned()),
            "registries" => {
                registries =
                    Some(value.parse().map_err(|_| {
                        format!("Julia returned an invalid registry count: {value}")
                    })?)
            }
            _ => {}
        }
    }

    Ok(Probe {
        version: version.ok_or_else(|| "Julia probe did not report its version".to_owned())?,
        active_project: active_project
            .ok_or_else(|| "Julia probe did not report its active project".to_owned())?,
        depot: depot.ok_or_else(|| "Julia probe did not report its depot".to_owned())?,
        registries: registries
            .ok_or_else(|| "Julia probe did not report reachable registries".to_owned())?,
    })
}

fn pkg_expression(command: &Command) -> Result<String, String> {
    let expression = match command {
        Command::Add { packages, develop } => {
            let specs = package_specs(packages)?;
            let operation = if *develop { "develop" } else { "add" };
            format!("import Pkg; Pkg.{operation}({specs})")
        }
        Command::Remove { packages } => {
            format!("import Pkg; Pkg.rm({})", string_vector(packages))
        }
        Command::Update { packages } if packages.is_empty() => {
            "import Pkg; Pkg.update()".to_owned()
        }
        Command::Update { packages } => {
            format!("import Pkg; Pkg.update({})", string_vector(packages))
        }
        Command::Instantiate => "import Pkg; Pkg.instantiate()".to_owned(),
        Command::Resolve => "import Pkg; Pkg.resolve()".to_owned(),
        Command::Precompile => "import Pkg; Pkg.precompile()".to_owned(),
        Command::Status => "import Pkg; Pkg.status()".to_owned(),
        Command::Gc => "import Pkg; Pkg.gc()".to_owned(),
        Command::Outdated => "import Pkg; Pkg.status(; outdated=true)".to_owned(),
        Command::Test { coverage, args } => format!(
            "import Pkg; Pkg.test(; coverage={}, test_args={})",
            coverage,
            string_vector(args)
        ),
        _ => return Err("this command cannot be executed through Julia Pkg".to_owned()),
    };
    Ok(expression)
}

fn package_specs(packages: &[String]) -> Result<String, String> {
    packages
        .iter()
        .map(|package| package_spec(package))
        .collect::<Result<Vec<_>, _>>()
        .map(|specs| format!("[{}]", specs.join(", ")))
}

fn package_spec(package: &str) -> Result<String, String> {
    if package.trim().is_empty() {
        return Err("package specifications cannot be empty".to_owned());
    }

    if is_url(package) {
        let (url, rev) = split_revision(package);
        let mut fields = vec![format!("url={}", julia_string(url))];
        if let Some(rev) = rev {
            fields.push(format!("rev={}", julia_string(rev)));
        }
        return Ok(format!("Pkg.PackageSpec({})", fields.join(", ")));
    }

    if looks_like_path(package) {
        let absolute = absolute_path(package)?;
        return Ok(format!(
            "Pkg.PackageSpec(path={})",
            julia_string(&absolute.to_string_lossy())
        ));
    }

    let (name, version) = split_version(package);
    if !valid_package_name(name) {
        return Err(format!("invalid Julia package name '{name}'"));
    }

    match version {
        Some("") => Err(format!("missing version after '@' in '{package}'")),
        Some(version) => Ok(format!(
            "Pkg.PackageSpec(name={}, version={})",
            julia_string(name),
            julia_string(version)
        )),
        None => Ok(format!("Pkg.PackageSpec(name={})", julia_string(name))),
    }
}

fn split_version(package: &str) -> (&str, Option<&str>) {
    package
        .split_once('@')
        .map_or((package, None), |(name, version)| (name, Some(version)))
}

fn split_revision(url: &str) -> (&str, Option<&str>) {
    url.rsplit_once('#')
        .map_or((url, None), |(url, rev)| (url, Some(rev)))
}

fn is_url(value: &str) -> bool {
    value.contains("://") || value.starts_with("git@")
}

fn looks_like_path(value: &str) -> bool {
    value == "."
        || value == ".."
        || value.starts_with("./")
        || value.starts_with("../")
        || value.starts_with('/')
        || value.starts_with("~/")
        || Path::new(value).exists()
}

fn absolute_path(value: &str) -> Result<PathBuf, String> {
    let path = if let Some(rest) = value.strip_prefix("~/") {
        let home = std::env::var_os("HOME")
            .ok_or_else(|| "cannot expand '~': HOME is not set".to_owned())?;
        PathBuf::from(home).join(rest)
    } else {
        PathBuf::from(value)
    };

    if path.is_absolute() {
        Ok(path)
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .map_err(|error| format!("cannot resolve package path: {error}"))
    }
}

fn valid_package_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn string_vector(values: &[String]) -> String {
    format!(
        "String[{}]",
        values
            .iter()
            .map(|value| julia_string(value))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn julia_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '$' => escaped.push_str("\\$"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character => escaped.push(character),
        }
    }
    escaped.push('"');
    escaped
}

fn command_description(command: &Command) -> &'static str {
    match command {
        Command::Add { develop: true, .. } => "developing packages",
        Command::Add { .. } => "adding packages",
        Command::Remove { .. } => "removing packages",
        Command::Update { .. } => "updating packages",
        Command::Instantiate => "instantiating the project",
        Command::Resolve => "resolving the project",
        Command::Precompile => "precompiling packages",
        Command::Status => "reading project status",
        Command::Gc => "collecting unused packages",
        Command::Outdated => "checking outdated packages",
        Command::Test { .. } => "running project tests",
        _ => "running the command",
    }
}

fn print_command(program: &OsStr, args: &[OsString]) {
    print!("{}", shell_quote(program));
    for arg in args {
        print!(" {}", shell_quote(arg));
    }
    println!();
}

fn shell_quote(value: &OsStr) -> String {
    let value = value.to_string_lossy();
    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "_-./=:".contains(character))
    {
        value.into_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\"'\"'"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_julia_strings() {
        assert_eq!(julia_string("A\"b\\c$d\n"), "\"A\\\"b\\\\c\\$d\\n\"");
    }

    #[test]
    fn creates_registry_package_specs() {
        assert_eq!(
            package_spec("Example").unwrap(),
            "Pkg.PackageSpec(name=\"Example\")"
        );
        assert_eq!(
            package_spec("Example@1.2").unwrap(),
            "Pkg.PackageSpec(name=\"Example\", version=\"1.2\")"
        );
    }

    #[test]
    fn creates_git_package_specs() {
        assert_eq!(
            package_spec("https://example.test/Repo.jl.git#feature").unwrap(),
            "Pkg.PackageSpec(url=\"https://example.test/Repo.jl.git\", rev=\"feature\")"
        );
    }

    #[test]
    fn escapes_untrusted_package_spec_fields() {
        assert_eq!(
            package_spec("Example@1.2\"; error($payload)\n#").unwrap(),
            "Pkg.PackageSpec(name=\"Example\", version=\"1.2\\\"; error(\\$payload)\\n#\")"
        );
        assert_eq!(
            package_spec("https://example.test/Repo.jl.git#feature\"; error($payload)").unwrap(),
            "Pkg.PackageSpec(url=\"https://example.test/Repo.jl.git\", rev=\"feature\\\"; error(\\$payload)\")"
        );
    }

    #[test]
    fn rejects_invalid_names() {
        assert_eq!(
            package_spec("bad-name").unwrap_err(),
            "invalid Julia package name 'bad-name'"
        );
    }

    #[test]
    fn run_passes_arguments_directly_to_julia() {
        let command = Command::Run {
            args: vec![OsString::from("script.jl"), OsString::from("--fast")],
        };
        let (args, _) = invocation(&command, Path::new("/tmp/demo")).unwrap();
        assert_eq!(
            args,
            vec![
                OsString::from("--project=/tmp/demo"),
                OsString::from("script.jl"),
                OsString::from("--fast")
            ]
        );
    }

    #[test]
    fn parses_environment_probe() {
        let probe = parse_probe(
            "version\t1.12.6\nproject\t/tmp/Demo/Project.toml\ndepot\t/home/me/.julia\nregistries\t2\n",
        )
        .unwrap();

        assert_eq!(
            probe,
            Probe {
                version: "1.12.6".to_owned(),
                active_project: "/tmp/Demo/Project.toml".to_owned(),
                depot: "/home/me/.julia".to_owned(),
                registries: 2,
            }
        );
    }

    #[test]
    fn rejects_incomplete_environment_probe() {
        assert_eq!(
            parse_probe("version\t1.12.6\n").unwrap_err(),
            "Julia probe did not report its active project"
        );
    }

    #[test]
    fn adds_canonical_test_scaffold() {
        let path = std::env::temp_dir().join(format!("jpm-test-scaffold-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("Project.toml"),
            "name = \"Demo\"\nuuid = \"00000000-0000-0000-0000-000000000000\"\n",
        )
        .unwrap();

        add_test_scaffold(&path, "Demo").unwrap();

        let tests = fs::read_to_string(path.join("test/runtests.jl")).unwrap();
        assert!(tests.contains("using Demo"));
        assert!(tests.contains("@testset \"Demo.jl\""));
        let project = fs::read_to_string(path.join("Project.toml")).unwrap();
        assert!(project.contains("[extras]"));
        assert!(project.contains("test = [\"Test\"]"));
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn builds_pkg_test_expression() {
        assert_eq!(
            pkg_expression(&Command::Test {
                coverage: true,
                args: vec!["fast".to_owned()]
            })
            .unwrap(),
            "import Pkg; Pkg.test(; coverage=true, test_args=String[\"fast\"])"
        );
    }

    #[test]
    fn escapes_pkg_string_arguments() {
        assert_eq!(
            pkg_expression(&Command::Remove {
                packages: vec!["Example\"); error($payload)\n#".to_owned()]
            })
            .unwrap(),
            "import Pkg; Pkg.rm(String[\"Example\\\"); error(\\$payload)\\n#\"])"
        );
    }
}
