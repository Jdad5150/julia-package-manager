use crate::{Config, julia};
use std::path::Path;

pub fn run(config: &Config, project: Option<&Path>) -> Result<(), String> {
    println!("jpm doctor\n");
    ok("jpm", env!("CARGO_PKG_VERSION"));

    match project {
        Some(path) => {
            ok("Project", &path.join("Project.toml").display().to_string());
            let manifest = path.join("Manifest.toml");
            if manifest.is_file() {
                ok("Manifest", &manifest.display().to_string());
            } else {
                warn(
                    "Manifest",
                    "not found; it will be created when dependencies are resolved",
                );
            }
        }
        None => warn(
            "Project",
            "none found; run 'jpm init' or use --project <PATH>",
        ),
    }

    let Some(probe) = julia::probe(config, project)? else {
        return Ok(());
    };

    ok("Julia", &probe.version);
    ok("Active project", &probe.active_project);
    ok("Depot", &probe.depot);
    if probe.registries == 0 {
        warn(
            "Registries",
            "none reachable; the General registry will be installed on first add",
        );
    } else {
        ok("Registries", &format!("{} reachable", probe.registries));
    }

    println!("\nEnvironment is ready.");
    Ok(())
}

fn ok(label: &str, detail: &str) {
    println!("[ok]   {label}: {detail}");
}

fn warn(label: &str, detail: &str) {
    println!("[warn] {label}: {detail}");
}
