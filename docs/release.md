# Releasing

Distribution is handled by [`cargo-dist`](https://github.com/axodotdev/cargo-dist)
(pinned to `0.31.0`) + GitHub Actions. Pushing a `vX.Y.Z` tag triggers
[`.github/workflows/release.yml`](.github/workflows/release.yml), which:

1. Builds binaries for macOS (`aarch64`, `x86_64`), Linux (`x86_64`), and
   Windows (`x86_64`).
2. Generates installers — shell, PowerShell, Homebrew, and an MSI — plus
   checksums, and attaches them to a GitHub Release.
3. Publishes the `soaim` Homebrew formula to the
   [`maximgorbatyuk/homebrew-tap`](https://github.com/maximgorbatyuk/homebrew-tap)
   repo, which is what `brew install soaim` pulls.

The pipeline is configured in [`dist-workspace.toml`](dist-workspace.toml).

## Cutting a release

```sh
./scripts/release.py 0.2.0    # explicit version
./scripts/release.py          # auto-increment the patch from Cargo.toml
```

The script ([`scripts/release.py`](scripts/release.py)) automates a `dev` → `main`
flow: it validates the version, checks that [`CHANGELOG.md`](CHANGELOG.md) has a
matching latest entry, runs `just verify`, commits and pushes `dev`, merges into
`main`, then creates and pushes the `vX.Y.Z` tag that starts the release. Add the
new `## [X.Y.Z]` section to `CHANGELOG.md` (as the top entry) before running it.

Monitor the run with `gh run list --workflow release.yml`.

## Prerequisites

| Requirement | For | Install |
|-------------|-----|---------|
| `gh`, authenticated | `scripts/release.py` | `brew install gh` then `gh auth login` |
| `just` | `just verify` (release gate) | `brew install just` |
| `cargo-dist` 0.31.0 | reproducing the pipeline locally (`just dist-plan`) | `cargo install cargo-dist@0.31.0` |
| `HOMEBREW_TAP_TOKEN` repo secret | the Homebrew publish job | GitHub PAT with write access to `maximgorbatyuk/homebrew-tap`, added in repo Settings → Secrets |

On every push and pull request, [`.github/workflows/ci.yml`](.github/workflows/ci.yml)
runs the format check, Clippy (warnings as errors), build, and tests on Linux and
Windows.