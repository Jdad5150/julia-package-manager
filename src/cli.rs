use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub julia: OsString,
    pub project: Option<PathBuf>,
    pub dry_run: bool,
    pub command: Command,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Init {
        path: Option<PathBuf>,
    },
    New {
        path: PathBuf,
    },
    Doctor,
    Use {
        channel: String,
    },
    Test {
        coverage: bool,
        args: Vec<String>,
    },
    Fmt {
        paths: Vec<PathBuf>,
    },
    Outdated,
    Tree,
    Why {
        package: String,
    },
    Add {
        packages: Vec<String>,
        develop: bool,
    },
    Remove {
        packages: Vec<String>,
    },
    Update {
        packages: Vec<String>,
    },
    Instantiate,
    Resolve,
    Precompile,
    Status,
    Gc,
    Run {
        args: Vec<OsString>,
    },
    Help,
    Version,
}

pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Config, String> {
    let mut args = args.into_iter().peekable();
    let mut julia = std::env::var_os("JPM_JULIA").unwrap_or_else(|| OsString::from("julia"));
    let mut project = std::env::var_os("JPM_PROJECT").map(PathBuf::from);
    let mut dry_run = false;

    let command = loop {
        let Some(arg) = args.next() else {
            break Command::Help;
        };
        let text = arg
            .to_str()
            .ok_or_else(|| "command-line options must be valid UTF-8".to_owned())?;

        match text {
            "--julia" => {
                julia = args
                    .next()
                    .ok_or_else(|| "--julia requires an executable".to_owned())?;
            }
            "--project" | "-p" => {
                project = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| "--project requires a path".to_owned())?,
                ));
            }
            "--dry-run" => dry_run = true,
            "--help" | "-h" | "help" => break Command::Help,
            "--version" | "-V" | "version" => break Command::Version,
            "init" => {
                let rest = collect_strings(args)?;
                if rest.len() > 1 {
                    return Err("usage: jpm init [PATH]".to_owned());
                }
                break Command::Init {
                    path: rest.first().map(PathBuf::from),
                };
            }
            "new" => {
                let rest = collect_strings(args)?;
                if rest.len() != 1 {
                    return Err("usage: jpm new <PACKAGE_PATH>".to_owned());
                }
                break Command::New {
                    path: PathBuf::from(&rest[0]),
                };
            }
            "doctor" => {
                ensure_empty("doctor", args)?;
                break Command::Doctor;
            }
            "use" => {
                let rest = collect_strings(args)?;
                if rest.len() != 1 {
                    return Err("usage: jpm use <JULIA_CHANNEL>".to_owned());
                }
                break Command::Use {
                    channel: rest[0].clone(),
                };
            }
            "test" => {
                let (coverage, args) = parse_test_args(collect_strings(args)?)?;
                break Command::Test { coverage, args };
            }
            "fmt" | "format" => {
                let paths = collect_strings(args)?
                    .into_iter()
                    .map(PathBuf::from)
                    .collect();
                break Command::Fmt { paths };
            }
            "outdated" => {
                ensure_empty("outdated", args)?;
                break Command::Outdated;
            }
            "tree" => {
                ensure_empty("tree", args)?;
                break Command::Tree;
            }
            "why" => {
                let rest = collect_strings(args)?;
                if rest.len() != 1 {
                    return Err("usage: jpm why <PACKAGE>".to_owned());
                }
                break Command::Why {
                    package: rest[0].clone(),
                };
            }
            "add" => {
                let mut packages = collect_strings(args)?;
                let develop = remove_flag(&mut packages, "--dev");
                require_packages("add", &packages)?;
                break Command::Add { packages, develop };
            }
            "remove" | "rm" => {
                let packages = collect_strings(args)?;
                require_packages("remove", &packages)?;
                break Command::Remove { packages };
            }
            "update" | "up" => {
                break Command::Update {
                    packages: collect_strings(args)?,
                };
            }
            "instantiate" | "install" => {
                ensure_empty("instantiate", args)?;
                break Command::Instantiate;
            }
            "resolve" => {
                ensure_empty("resolve", args)?;
                break Command::Resolve;
            }
            "precompile" => {
                ensure_empty("precompile", args)?;
                break Command::Precompile;
            }
            "status" | "st" | "list" | "ls" => {
                ensure_empty("status", args)?;
                break Command::Status;
            }
            "gc" => {
                ensure_empty("gc", args)?;
                break Command::Gc;
            }
            "run" => {
                let mut run_args: Vec<OsString> = args.collect();
                if run_args.first().is_some_and(|arg| arg == "--") {
                    run_args.remove(0);
                }
                if run_args.is_empty() {
                    return Err("usage: jpm run <SCRIPT | JULIA_ARGS...>".to_owned());
                }
                break Command::Run { args: run_args };
            }
            unknown if unknown.starts_with('-') => {
                return Err(format!("unknown option '{unknown}'\n\n{}", help()));
            }
            unknown => {
                return Err(format!("unknown command '{unknown}'\n\n{}", help()));
            }
        }
    };

    Ok(Config {
        julia,
        project,
        dry_run,
        command,
    })
}

fn collect_strings(args: impl IntoIterator<Item = OsString>) -> Result<Vec<String>, String> {
    args.into_iter()
        .map(|arg| {
            arg.into_string()
                .map_err(|_| "package specifications must be valid UTF-8".to_owned())
        })
        .collect()
}

fn ensure_empty(command: &str, mut args: impl Iterator<Item = OsString>) -> Result<(), String> {
    if let Some(arg) = args.next() {
        return Err(format!(
            "unexpected argument '{}' for '{command}'",
            arg.to_string_lossy()
        ));
    }
    Ok(())
}

fn require_packages(command: &str, packages: &[String]) -> Result<(), String> {
    if packages.is_empty() {
        Err(format!("usage: jpm {command} <PACKAGE...>"))
    } else {
        Ok(())
    }
}

fn remove_flag(values: &mut Vec<String>, flag: &str) -> bool {
    if let Some(index) = values.iter().position(|value| value == flag) {
        values.remove(index);
        true
    } else {
        false
    }
}

fn parse_test_args(values: Vec<String>) -> Result<(bool, Vec<String>), String> {
    let mut coverage = false;
    let mut passthrough = false;
    let mut args = Vec::new();

    for value in values {
        if passthrough {
            args.push(value);
        } else {
            match value.as_str() {
                "--" => passthrough = true,
                "--coverage" => coverage = true,
                option if option.starts_with('-') => {
                    return Err(format!(
                        "unknown option '{option}' for 'test'; pass test arguments after '--'"
                    ));
                }
                _ => args.push(value),
            }
        }
    }

    Ok((coverage, args))
}

pub fn help() -> &'static str {
    "\
jpm - a fast, friendly interface to Julia's package manager

USAGE:
    jpm [OPTIONS] <COMMAND>

OPTIONS:
    -p, --project <PATH>   Use a specific Julia project
        --julia <PATH>     Julia executable (or set JPM_JULIA)
        --dry-run          Print the Julia command without running it
    -h, --help             Print help
    -V, --version          Print version

COMMANDS:
    init [PATH]            Create a Julia environment
    new <PACKAGE_PATH>     Generate a Julia package
    doctor                 Diagnose the local Julia environment
    use <JULIA_CHANNEL>    Select a Julia version with Juliaup
    test [--coverage]      Run project tests
    fmt [PATHS...]         Format Julia source with JuliaFormatter
    outdated               Show packages with available updates
    tree                   Show the dependency tree
    why <PACKAGE>           Explain why a package is installed
    add [--dev] <PKG...>   Add registry, URL, or local packages
    remove <PKG...>        Remove packages (alias: rm)
    update [PKG...]        Update all or selected packages (alias: up)
    instantiate            Install dependencies from the manifest
    resolve                Resolve project compatibility
    precompile             Precompile project dependencies
    status                 Show project dependencies (alias: st)
    gc                     Remove unused package artifacts
    run <ARGS...>          Run Julia in the project environment

PACKAGE SPECS:
    Example                Registry package
    Example@1.2            Registry package with a version constraint
    ./LocalPackage         Local package path
    https://host/repo.git  Git repository URL
"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(args: &[&str]) -> Result<Config, String> {
        parse(args.iter().map(OsString::from))
    }

    #[test]
    fn parses_global_options_and_add() {
        let config = parse_str(&[
            "--julia",
            "/opt/julia",
            "--project",
            "demo",
            "--dry-run",
            "add",
            "--dev",
            "../Local",
        ])
        .unwrap();

        assert_eq!(config.julia, "/opt/julia");
        assert_eq!(config.project, Some(PathBuf::from("demo")));
        assert!(config.dry_run);
        assert_eq!(
            config.command,
            Command::Add {
                packages: vec!["../Local".to_owned()],
                develop: true
            }
        );
    }

    #[test]
    fn aliases_are_supported() {
        assert_eq!(
            parse_str(&["rm", "Example"]).unwrap().command,
            Command::Remove {
                packages: vec!["Example".to_owned()]
            }
        );
        assert_eq!(parse_str(&["st"]).unwrap().command, Command::Status);
    }

    #[test]
    fn package_commands_require_arguments() {
        assert_eq!(
            parse_str(&["add"]).unwrap_err(),
            "usage: jpm add <PACKAGE...>"
        );
    }

    #[test]
    fn parses_new_and_doctor() {
        assert_eq!(
            parse_str(&["new", "DemoPackage"]).unwrap().command,
            Command::New {
                path: PathBuf::from("DemoPackage")
            }
        );
        assert_eq!(parse_str(&["doctor"]).unwrap().command, Command::Doctor);
        assert_eq!(
            parse_str(&["new"]).unwrap_err(),
            "usage: jpm new <PACKAGE_PATH>"
        );
    }

    #[test]
    fn parses_dependency_insight_commands() {
        assert_eq!(parse_str(&["outdated"]).unwrap().command, Command::Outdated);
        assert_eq!(parse_str(&["tree"]).unwrap().command, Command::Tree);
        assert_eq!(
            parse_str(&["why", "Compat"]).unwrap().command,
            Command::Why {
                package: "Compat".to_owned()
            }
        );
        assert_eq!(parse_str(&["why"]).unwrap_err(), "usage: jpm why <PACKAGE>");
    }

    #[test]
    fn parses_project_workflow_commands() {
        assert_eq!(
            parse_str(&["use", "1.12"]).unwrap().command,
            Command::Use {
                channel: "1.12".to_owned()
            }
        );
        assert_eq!(
            parse_str(&["test", "--coverage", "--", "fast"])
                .unwrap()
                .command,
            Command::Test {
                coverage: true,
                args: vec!["fast".to_owned()]
            }
        );
        assert_eq!(
            parse_str(&["fmt", "src", "test/runtests.jl"])
                .unwrap()
                .command,
            Command::Fmt {
                paths: vec![PathBuf::from("src"), PathBuf::from("test/runtests.jl")]
            }
        );
    }

    #[test]
    fn test_separator_preserves_test_arguments() {
        assert_eq!(
            parse_str(&["test", "--coverage", "--", "--coverage", "--fast"])
                .unwrap()
                .command,
            Command::Test {
                coverage: true,
                args: vec!["--coverage".to_owned(), "--fast".to_owned()]
            }
        );
        assert_eq!(
            parse_str(&["test", "--unknown"]).unwrap_err(),
            "unknown option '--unknown' for 'test'; pass test arguments after '--'"
        );
    }
}
