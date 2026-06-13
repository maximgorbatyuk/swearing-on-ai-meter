# Privacy Policy

Last updated: 2026-06-13

This Privacy Policy explains how `soaim` ("Swearing on AI Meter") handles information when you use the open source project, its source code repository, official release artifacts, and project website.

`soaim` is a local, offline command-line and terminal-UI tool. It scans your own AI-coding prompt history on your machine, counts swear words, caches the results in a local SQLite database, and shows charts in your terminal. Every statistic is broken down per app.

## Scope

This policy applies to:

- the official `soaim` source repository;
- official binaries and release artifacts published by the project maintainer; and
- documentation distributed with the project.

This policy does not automatically apply to forks, third-party builds, package mirrors, or modified versions of the software. Those distributions may follow different privacy practices.

## Information `soaim` processes locally

When you run `soaim`, the software reads **user-authored prompts** from each AI tool's local on-disk history, including:

- prompt text you wrote to your AI coding agents;
- timestamps and message identifiers used to deduplicate and bucket prompts;
- the source paths and database location it resolves on your machine.

From this it computes swear counts and aggregates (per day, per hour, per weekday, per app). All processing happens locally on your device.

`soaim` reads from the following local sources by default — Claude Code (`~/.claude/projects/**/*.jsonl`), Codex CLI (`~/.codex/...`), and opencode (`~/.local/share/opencode/opencode.db`, opened strictly read-only) — and, only when you explicitly configure them, Claude Desktop and Codex App.

## What the project does not do

Based on the published project documentation and repository contents at the time of this policy:

- `soaim` does not require a user account;
- `soaim` does not operate a hosted service;
- `soaim` runs **fully offline** and makes **no network calls**;
- `soaim` does not read your AI agents' replies — only your own prompts;
- `soaim` is **read-only** and never writes to any tool's history or files;
- `soaim` does not send your prompts, counts, or any other data to the project maintainer;
- `soaim` does not include advertising or behavioral tracking.

If the project adds optional networked features in a future release, this policy should be updated before or when those features are released.

## Where processing happens

The intended privacy model for `soaim` is local processing on the user's machine. Prompt reading, swear counting, and aggregation are all performed on your device. Results are cached in a local SQLite database (default: `~/.local/share/soaim/soaim.db`). External SQLite stores it reads (such as opencode's) are opened read-only so a running instance is never disturbed.

## Sharing and disclosure

The project maintainer does not receive your data from the software.

Information may still be disclosed by you or your environment if you choose to:

- share `soaim` output or screenshots;
- paste command or dashboard output into issue trackers, chats, or AI tools; or
- run the software on systems monitored by your employer, hosting provider, or device management tools.

Please review generated output before sharing it. Output may indirectly reveal your activity patterns.

## Command-line and local environment considerations

Like many CLI tools, use of `soaim` may expose limited operational data to your local environment, including:

- shell history containing commands you ran;
- terminal scrollback or log capture tools; and
- operating system file access records.

These behaviors are generally controlled by your shell, operating system, and development environment rather than by the project maintainer.

## Third-party services and distribution channels

The project may be distributed or hosted through third-party services, including:

- GitHub, for source hosting, release distribution, and the project website (GitHub Pages); and
- GitHub Actions, for release automation; and
- Homebrew, for package installation on macOS.

If you download the software, browse the repository, install via Homebrew, or interact with GitHub-hosted project resources, those third parties may collect information under their own privacy policies and terms. The `soaim` project does not control those third-party practices.

## Website

The project website is served as static files via GitHub Pages and does not use cookies or analytics. To display the latest release, the page makes a single client-side request from your browser to the public GitHub API (`api.github.com`); that request is subject to GitHub's own privacy policy. No information is sent to the project maintainer.

## Data retention

Because `soaim` is designed for local use, the project maintainer does not retain your data through the software. The only data `soaim` stores is its local SQLite cache on your machine. You can delete that database (and any related local files) at any time — `soaim` will simply re-ingest from your existing prompt history on the next run.

## Security

`soaim` is intended to minimize privacy risk by operating locally, offline, and read-only, without requiring a hosted backend.

However, no software environment is completely risk-free. You are responsible for protecting access to your machine and to the local prompt-history files and cache that `soaim` reads and writes.

## Open source development and forks

Because `soaim` is open source, anyone may inspect the source code and, subject to the license, create modified versions. Modified versions, forks, unofficial packages, or downstream distributions may behave differently from the official project and may introduce new data practices.

You should review the source, release notes, and privacy terms for any non-official distribution you use.

## Children's privacy

`soaim` is a developer tool and is not directed to children.

## Changes to this policy

This Privacy Policy may be updated as the project evolves. Material changes should be reflected in the repository so users can review the current version before using new features.

## Contact

For questions about this Privacy Policy or the official project, please use the project's public repository:

- `https://github.com/maximgorbatyuk/swearing-on-ai-meter`

If you need to report a privacy or security concern, please open an appropriate issue or contact channel provided through the official repository.
