# jpm Roadmap

## Product Signal

The core value is cohesion, not replacing Julia's package tooling.

During the first real user test, `jpm` made it possible to write Julia in a
script and run it without the usual mental overhead. Preserve that direct
workflow:

```console
jpm init
jpm add Package
jpm run script.jl
```

New features should reduce context switching and keep Julia, Pkg, and Juliaup
authoritative.

## Next Milestone: Public Alpha

Prioritize release engineering before adding more features:

- [x] Create and publish the Git repository with complete package metadata.
- [x] Run CI on macOS, Linux, and Windows.
- [x] Test against Julia LTS and the current stable release.
- [x] Add real Julia integration tests, including paths with spaces.
- [x] Cover missing tools, offline operation, and failed package operations.
- [x] Audit Julia command generation and process execution for injection risks.
- [x] Automate binary releases and publish checksums.
- [x] Document that `jpm fmt` bootstraps JuliaFormatter.
- [x] Document that `jpm use` requires Juliaup.

Release the result as `0.4.0-alpha.1`.

## Release Stages

- **Alpha:** Validate installation and real-project workflows publicly.
- **Beta:** Begin after 5-10 external users have exercised representative
  projects.
- **1.0:** Release when the CLI contract is stable and project or manifest
  mutations have strong data-loss safeguards.
