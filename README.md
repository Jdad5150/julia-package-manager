# jpm

**A fast, predictable command-line interface for Julia package management.**

[![CI](https://github.com/Jdad5150/julia-package-manager/actions/workflows/ci.yml/badge.svg)](https://github.com/Jdad5150/julia-package-manager/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Jdad5150/julia-package-manager?include_prereleases)](https://github.com/Jdad5150/julia-package-manager/releases)
[![License](https://img.shields.io/github/license/Jdad5150/julia-package-manager)](LICENSE)
[![Rust 1.85+](https://img.shields.io/badge/Rust-1.85%2B-000000?logo=rust)](https://www.rust-lang.org/)

`jpm` brings Julia's common package and project operations into one cohesive
shell interface. Julia's built-in `Pkg` remains the resolver and source of
truth; `jpm` focuses on workflow, discoverability, and useful diagnostics.

> [!NOTE]
> `jpm` is currently in public alpha. Core workflows are tested on Linux,
> macOS, and Windows against Julia LTS and the latest stable Julia release.
> Command behavior may still evolve before 1.0.

## Quick Start

Create an environment, add a dependency, and run a Julia script:

```console
$ mkdir analysis && cd analysis
$ jpm init
$ jpm add DataFrames
$ jpm run analysis.jl
```

`jpm` searches the current directory and its parents for `Project.toml`, so
commands work from anywhere inside a project.

## Installation

Julia must be installed and available on `PATH`. [Juliaup](https://julialang.org/install/)
is recommended and is required only for `jpm use`.

### Prebuilt Binaries

Download the archive for your platform from the
[current alpha release](https://github.com/Jdad5150/julia-package-manager/releases/tag/v0.4.0-alpha.1),
extract it, and place `jpm` (or `jpm.exe`) in a directory on `PATH`.

| Platform | Release asset |
| --- | --- |
| Linux x86-64 | `jpm-0.4.0-alpha.1-x86_64-unknown-linux-gnu.tar.gz` |
| macOS Intel | `jpm-0.4.0-alpha.1-x86_64-apple-darwin.tar.gz` |
| Windows x86-64 | `jpm-0.4.0-alpha.1-x86_64-pc-windows-msvc.zip` |

Each release includes `SHA256SUMS` for download verification. Apple Silicon
users should currently install with Cargo for a native binary.

### Install With Cargo

Rust 1.85 or newer is required:

```console
cargo install --git https://github.com/Jdad5150/julia-package-manager \
  --tag v0.4.0-alpha.1
```

To install a local checkout:

```console
cargo install --path .
```

## Core Workflows

### Manage an Environment

```console
jpm init
jpm add HTTP JSON3
jpm status
jpm outdated
jpm update
```

### Create and Test a Package

```console
jpm new MyPackage
cd MyPackage
jpm use 1.12
jpm doctor
jpm test
jpm fmt
```

Package generation delegates to `Pkg.generate`, preserving Julia-compatible
UUIDs and source layout. `jpm` adds the standard `Test` target and a
`test/runtests.jl` scaffold.

### Understand Dependencies

```console
$ jpm why Preferences
Preferences v1.5.2 is installed because:

Project
`-- JSON3 v1.14.3
    `-- PrecompileTools v1.3.4
        `-- Preferences v1.5.2
```

Use `jpm tree` for the complete resolved graph. A `(*)` suffix marks a package
whose subtree was already displayed beneath the same direct dependency.

## Command Reference

### Projects and Tooling

| Command | Description |
| --- | --- |
| `jpm init [PATH]` | Create a minimal Julia environment |
| `jpm new <PACKAGE_PATH>` | Generate a Julia package with tests |
| `jpm doctor` | Check Julia, the active project, depot, manifest, and registries |
| `jpm use <JULIA_CHANNEL>` | Install and select a Juliaup channel for the project |
| `jpm test [--coverage] [-- ARGS...]` | Run project tests through `Pkg.test` |
| `jpm fmt [PATHS...]` | Format Julia source with JuliaFormatter |
| `jpm run <ARGS...>` | Run Julia with the project activated |

### Dependencies

| Command | Description |
| --- | --- |
| `jpm add <PKG...>` | Add registry, Git, or local packages |
| `jpm add --dev <PKG...>` | Develop packages using local checkouts |
| `jpm remove <PKG...>` | Remove direct dependencies |
| `jpm update [PKG...]` | Update all or selected dependencies |
| `jpm instantiate` | Install dependencies from `Manifest.toml` |
| `jpm resolve` | Resolve project compatibility |
| `jpm precompile` | Precompile project dependencies |
| `jpm status` | Display project status |
| `jpm outdated` | Show dependencies with compatible updates |
| `jpm tree` | Display the resolved dependency graph |
| `jpm why <PACKAGE>` | Explain why a package is installed |
| `jpm gc` | Garbage-collect unused package data |

Run `jpm --help` for the complete CLI help.

## Configuration

| Option or variable | Purpose |
| --- | --- |
| `-p, --project <PATH>` | Select a Julia project explicitly |
| `JPM_PROJECT` | Set the default project path |
| `--julia <PATH>` | Select a Julia executable |
| `JPM_JULIA` | Set the default Julia executable |
| `JPM_JULIAUP` | Set the Juliaup executable used by `jpm use` |
| `--dry-run` | Print the exact Julia or Juliaup invocation |

For example:

```console
JPM_JULIA=/opt/julia/bin/julia jpm status
jpm --project ./environments/dev --dry-run add Example@1.2
```

## How It Works

- Package mutations are delegated directly to Julia's `Pkg` API.
- Julia is launched without user startup or history files for repeatable
  package operations.
- User-provided package specifications and arguments are escaped before being
  embedded in Julia expressions.
- `jpm fmt` installs JuliaFormatter into the shared `@jpm-tools` environment,
  keeping formatter dependencies out of the project.
- `jpm use` stores a Juliaup directory override that also applies to direct
  `julia` commands inside the project.

## Roadmap

The current priority is validating real projects and stabilizing the CLI
contract. Planned work includes:

1. Machine-readable output for editors and CI.
2. Actionable compatibility and resolution diagnostics.
3. Lockfile diff and dependency audit views.
4. Workspace support for multi-package repositories.
5. Fast registry search backed by a local Rust index.

See [ROADMAP.md](ROADMAP.md) for release criteria and project direction.

## Development

```console
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

Run the real Julia integration workflow with:

```console
JPM_RUN_JULIA_INTEGRATION=1 cargo test --locked
```

## License

Licensed under the [MIT License](LICENSE).
