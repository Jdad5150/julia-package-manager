use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(label: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("jpm-cli-{label}-{}-{nonce}", std::process::id()))
}

#[test]
fn help_describes_core_workflow() {
    let output = Command::new(env!("CARGO_BIN_EXE_jpm"))
        .arg("--help")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("jpm [OPTIONS] <COMMAND>"));
    assert!(stdout.contains("instantiate"));
}

#[test]
fn init_and_dry_run_work_end_to_end() {
    let directory = temp_dir("workflow");

    let init = Command::new(env!("CARGO_BIN_EXE_jpm"))
        .args(["init", directory.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(init.status.success());
    assert_eq!(
        fs::read_to_string(directory.join("Project.toml")).unwrap(),
        "[deps]\n"
    );

    let dry_run = Command::new(env!("CARGO_BIN_EXE_jpm"))
        .args([
            "--project",
            directory.to_str().unwrap(),
            "--dry-run",
            "add",
            "Example@1.2",
        ])
        .output()
        .unwrap();
    assert!(dry_run.status.success());
    let stdout = String::from_utf8(dry_run.stdout).unwrap();
    assert!(stdout.contains("Pkg.add"));
    assert!(stdout.contains("Example"));
    assert!(stdout.contains("version=\"1.2\""));

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn new_and_doctor_dry_runs_are_inspectable() {
    let directory = temp_dir("new");
    fs::create_dir_all(&directory).unwrap();
    let new_output = Command::new(env!("CARGO_BIN_EXE_jpm"))
        .args([
            "--dry-run",
            "new",
            directory.join("DemoPackage").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(new_output.status.success());
    let stdout = String::from_utf8(new_output.stdout).unwrap();
    assert!(stdout.contains("Pkg.generate"));
    assert!(stdout.contains("DemoPackage"));

    let doctor_output = Command::new(env!("CARGO_BIN_EXE_jpm"))
        .args(["--dry-run", "doctor"])
        .current_dir(&directory)
        .output()
        .unwrap();
    assert!(doctor_output.status.success());
    let stdout = String::from_utf8(doctor_output.stdout).unwrap();
    assert!(stdout.contains("[ok]   jpm:"));
    assert!(stdout.contains("[warn] Project: none found"));
    assert!(stdout.contains("reachable_registries"));

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn dependency_insights_support_dry_run() {
    let directory = temp_dir("insights");
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("Project.toml"), "[deps]\n").unwrap();

    for args in [
        vec!["--dry-run", "tree"],
        vec!["--dry-run", "why", "Compat"],
        vec!["--dry-run", "outdated"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_jpm"))
            .args(args)
            .current_dir(&directory)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(String::from_utf8(output.stdout).unwrap().contains("julia"));
    }

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn project_workflow_supports_dry_run() {
    let directory = temp_dir("workflow-v04");
    fs::create_dir_all(directory.join("src")).unwrap();
    fs::write(directory.join("Project.toml"), "[deps]\n").unwrap();
    fs::write(directory.join("src/Demo.jl"), "module Demo\nend\n").unwrap();

    let cases = [
        vec!["--dry-run", "use", "1.12"],
        vec!["--dry-run", "test", "--coverage", "--", "fast"],
        vec!["--dry-run", "fmt", "src"],
    ];
    for args in cases {
        let output = Command::new(env!("CARGO_BIN_EXE_jpm"))
            .args(args)
            .current_dir(&directory)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!output.stdout.is_empty());
    }

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn missing_tools_have_actionable_errors() {
    let missing = temp_dir("missing-tool").join("julia");
    let output = Command::new(env!("CARGO_BIN_EXE_jpm"))
        .args(["--julia", missing.to_str().unwrap(), "doctor"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("install Julia or set JPM_JULIA"));

    let directory = temp_dir("missing-juliaup");
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("Project.toml"), "[deps]\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_jpm"))
        .args(["use", "release"])
        .current_dir(&directory)
        .env("JPM_JULIAUP", &missing)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Juliaup was not found"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn real_julia_workflow_supports_spaces_offline_and_failures() {
    if std::env::var_os("JPM_RUN_JULIA_INTEGRATION").is_none() {
        return;
    }

    let directory = temp_dir("real workflow with spaces");
    fs::create_dir_all(&directory).unwrap();
    let project = directory.join("DemoPackage");
    let depot = integration_depot(&directory);

    assert_success(
        Command::new(env!("CARGO_BIN_EXE_jpm"))
            .args(["new", project.to_str().unwrap()])
            .env("JULIA_DEPOT_PATH", &depot),
    );
    assert_success(jpm(&project, &["doctor"]).env("JULIA_DEPOT_PATH", &depot));
    assert_success(jpm(&project, &["test"]).env("JULIA_DEPOT_PATH", &depot));

    let script = project.join("script with spaces.jl");
    fs::write(&script, "println(\"jpm-run-ok\")\n").unwrap();
    let run = assert_success(
        jpm(&project, &["run", script.to_str().unwrap()])
            .env("JULIA_DEPOT_PATH", &depot)
            .env("JULIA_PKG_OFFLINE", "true"),
    );
    assert!(String::from_utf8_lossy(&run.stdout).contains("jpm-run-ok"));

    let status = assert_success(
        jpm(&project, &["status"])
            .env("JULIA_DEPOT_PATH", &depot)
            .env("JULIA_PKG_OFFLINE", "true"),
    );
    assert!(String::from_utf8_lossy(&status.stdout).contains("DemoPackage"));

    let tree = assert_success(
        jpm(&project, &["tree"])
            .env("JULIA_DEPOT_PATH", &depot)
            .env("JULIA_PKG_OFFLINE", "true"),
    );
    assert!(String::from_utf8_lossy(&tree.stdout).contains("No project dependencies"));

    let failed = jpm(&project, &["add", "./definitely-missing-local-package"])
        .env("JULIA_DEPOT_PATH", &depot)
        .env("JULIA_PKG_OFFLINE", "true")
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("Julia failed while adding packages"));

    fs::remove_dir_all(directory).unwrap();
}

fn jpm(project: &Path, args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_jpm"));
    command.arg("--project").arg(project).args(args);
    command
}

fn integration_depot(directory: &Path) -> std::ffi::OsString {
    let writable = directory.join("depot");
    fs::create_dir_all(&writable).unwrap();
    let mut paths = vec![writable];

    if let Some(existing) = std::env::var_os("JULIA_DEPOT_PATH") {
        paths.extend(std::env::split_paths(&existing));
    } else if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))
    {
        paths.push(std::path::PathBuf::from(home).join(".julia"));
    }

    std::env::join_paths(paths).unwrap()
}

fn assert_success(command: &mut Command) -> std::process::Output {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}
