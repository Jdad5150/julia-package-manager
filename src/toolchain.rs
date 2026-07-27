use crate::Config;
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::Command;

pub fn select(config: &Config, project: &Path, channel: &str) -> Result<(), String> {
    validate_channel(channel)?;
    let juliaup = std::env::var_os("JPM_JULIAUP").unwrap_or_else(|| OsString::from("juliaup"));
    let add_args = [OsString::from("add"), OsString::from(channel)];
    let override_args = [
        OsString::from("override"),
        OsString::from("set"),
        OsString::from("--path"),
        project.as_os_str().to_owned(),
        OsString::from(channel),
    ];

    if config.dry_run {
        print_command(&juliaup, &add_args);
        print_command(&juliaup, &override_args);
        return Ok(());
    }

    run(&juliaup, &add_args, "installing the Julia channel")?;
    run(
        &juliaup,
        &override_args,
        "setting the project Julia channel",
    )?;
    println!("Using Julia channel {channel} for {}", project.display());
    Ok(())
}

fn validate_channel(channel: &str) -> Result<(), String> {
    if channel.is_empty()
        || channel
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
    {
        Err(format!("invalid Julia channel '{channel}'"))
    } else {
        Ok(())
    }
}

fn run(program: &OsStr, args: &[OsString], description: &str) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                "Juliaup was not found; install it from https://julialang.org/install/ or set JPM_JULIAUP"
                    .to_owned()
            } else {
                format!("could not start Juliaup: {error}")
            }
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "Juliaup failed while {description}{}",
            status
                .code()
                .map(|code| format!(" (exit code {code})"))
                .unwrap_or_default()
        ))
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
    fn validates_juliaup_channels() {
        assert!(validate_channel("1.12").is_ok());
        assert!(validate_channel("release").is_ok());
        assert_eq!(
            validate_channel("bad channel").unwrap_err(),
            "invalid Julia channel 'bad channel'"
        );
    }
}
