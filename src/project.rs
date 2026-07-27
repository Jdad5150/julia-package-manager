use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn discover(explicit: Option<&Path>) -> Result<PathBuf, String> {
    discover_optional(explicit)?
        .ok_or_else(|| "no Julia project found; run 'jpm init' or pass --project <PATH>".to_owned())
}

pub fn discover_optional(explicit: Option<&Path>) -> Result<Option<PathBuf>, String> {
    if let Some(path) = explicit {
        return validate(path).map(Some);
    }

    let cwd =
        env::current_dir().map_err(|error| format!("cannot read current directory: {error}"))?;
    for directory in cwd.ancestors() {
        if directory.join("Project.toml").is_file() {
            return Ok(Some(directory.to_path_buf()));
        }
    }

    Ok(None)
}

pub fn init(path: Option<&Path>) -> Result<(), String> {
    let path = path.unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(path)
        .map_err(|error| format!("cannot create '{}': {error}", path.display()))?;

    let project_file = path.join("Project.toml");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&project_file)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                format!("'{}' already exists", project_file.display())
            } else {
                format!("cannot create '{}': {error}", project_file.display())
            }
        })?;

    file.write_all(b"[deps]\n")
        .map_err(|error| format!("cannot write '{}': {error}", project_file.display()))?;
    println!("Initialized Julia environment in {}", path.display());
    Ok(())
}

fn validate(path: &Path) -> Result<PathBuf, String> {
    let project_file = if path.is_file() {
        path.to_path_buf()
    } else {
        path.join("Project.toml")
    };

    if !project_file.is_file() {
        return Err(format!(
            "'{}' is not a Julia project (Project.toml not found)",
            path.display()
        ));
    }

    Ok(project_file
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("jpm-{label}-{}-{nonce}", std::process::id()))
    }

    #[test]
    fn init_creates_minimal_project() {
        let directory = temp_dir("init");
        init(Some(&directory)).unwrap();
        assert_eq!(
            fs::read_to_string(directory.join("Project.toml")).unwrap(),
            "[deps]\n"
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn validates_directory_and_file_paths() {
        let directory = temp_dir("validate");
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("Project.toml"), "[deps]\n").unwrap();

        assert_eq!(validate(&directory).unwrap(), directory);
        assert_eq!(
            validate(&directory.join("Project.toml")).unwrap(),
            directory
        );
        fs::remove_dir_all(directory).unwrap();
    }
}
