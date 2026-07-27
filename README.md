# jpm

`jpm` is a fast, friendly command-line interface for Julia's built-in package
manager. It keeps Julia's `Pkg` resolver as the source of truth and improves the
day-to-day developer experience around it.

## Why

Julia package management is powerful, but common tasks alternate between the
Julia REPL, `Pkg` mode, shell commands, and project activation. `jpm` provides
one predictable shell interface:

```console
jpm init
jpm use 1.12
jpm new MyPackage
jpm doctor
jpm test
jpm fmt
jpm add HTTP JSON3
jpm tree
jpm why Parsers
jpm outdated
jpm add Example@1.2
jpm add --dev ../MyLocalPackage
jpm status
jpm run test/runtests.jl
```

`jpm` searches the current directory and its parents for `Project.toml`, so
commands work naturally from anywhere inside a project.

## Install From Source

Prerequisites:

- Rust 1.85 or newer
- Julia on `PATH`

```console
cargo install --path .
```

If Julia is installed somewhere unusual, set `JPM_JULIA` or pass `--julia`:

```console
JPM_JULIA=/opt/julia/bin/julia jpm status
jpm --julia /opt/julia/bin/julia status
```

## Commands

| Command | Purpose |
| --- | --- |
| `jpm init [PATH]` | Create a minimal Julia environment |
| `jpm new <PACKAGE_PATH>` | Generate a Julia package with canonical metadata and layout |
| `jpm doctor` | Diagnose Julia, project, manifest, depot, and registries |
| `jpm use <JULIA_CHANNEL>` | Install and select a Juliaup channel for the project |
| `jpm test [--coverage] [-- ARGS...]` | Run project tests through `Pkg.test` |
| `jpm fmt [PATHS...]` | Format Julia source with an isolated JuliaFormatter install |
| `jpm outdated` | Show direct dependencies with available compatible updates |
| `jpm tree` | Display the resolved dependency graph as a tree |
| `jpm why <PACKAGE>` | Show why a direct or transitive package is installed |
| `jpm add <PKG...>` | Add registry, Git, or local packages |
| `jpm add --dev <PKG...>` | Develop packages using local checkouts |
| `jpm remove <PKG...>` | Remove direct dependencies |
| `jpm update [PKG...]` | Update all or selected dependencies |
| `jpm instantiate` | Install dependencies from `Manifest.toml` |
| `jpm resolve` | Resolve project compatibility |
| `jpm precompile` | Precompile dependencies |
| `jpm status` | Display project status |
| `jpm gc` | Garbage-collect unused package data |
| `jpm run <ARGS...>` | Run Julia with the project activated |

Use `--project PATH` to select a project explicitly and `--dry-run` to inspect
the exact Julia invocation.

## Environments and Packages

Use `jpm init` when you need an environment for scripts, analysis, or an
application:

```console
mkdir analysis
cd analysis
jpm init
jpm add DataFrames
```

Use `jpm new` when you are authoring a reusable Julia package:

```console
jpm new MyPackage
cd MyPackage
jpm use 1.12
jpm doctor
jpm test
jpm fmt
```

Package generation delegates to Julia's `Pkg.generate`, so project UUIDs and
source layout remain compatible with the Julia toolchain. `jpm` also adds the
standard `Test` target and `test/runtests.jl` scaffold.

`jpm use` delegates installation and project selection to Juliaup. Juliaup
stores the directory override in its user configuration, so it applies to both
`jpm` and direct `julia` commands run within the project.

`jpm fmt` keeps JuliaFormatter in the shared `@jpm-tools` environment rather
than adding formatter dependencies to the project. The first invocation
installs the formatter; later runs reuse it.

## Dependency Insights

Inspect a resolved environment without switching into Julia's package REPL:

```console
$ jpm why Preferences
Preferences v1.5.2 is installed because:

Project
`-- JSON3 v1.14.3
    `-- PrecompileTools v1.3.4
        `-- Preferences v1.5.2
```

`jpm tree` starts from direct project dependencies and includes transitive
packages and Julia standard libraries. A `(*)` suffix marks a package whose
subtree was already shown earlier under the same direct dependency.

## Product Direction

`jpm` is intentionally a reliable frontend to `Pkg`, not a new resolver.
Likely next steps are:

1. Machine-readable output for editors and CI.
2. Actionable compatibility and resolution diagnostics.
3. Lockfile diff and dependency audit views.
4. Workspace support for multi-package Julia repositories.
5. Fast registry search with a local Rust index.

## Development

```console
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
