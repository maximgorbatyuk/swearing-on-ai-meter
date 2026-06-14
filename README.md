# Swearing on AI Meter (`soaim`)

"Swearing on AI Meter" is a Rust CLI/TUI that scans local AI-coding
prompt history across AI coding apps and counts swear words, caches results in its own
SQLite database, and renders live terminal charts. Every statistic is also
broken down per app.

Apps that are supported:

- Claude Code
- Codex CLI
- opencode
- Claude Desktop
- Codex App

## How to use it

### Homebrew

```sh
brew install maximgorbatyuk/tap/soaim

# launch
soaim
```

## License

MIT — see `LICENSE`.

## Contributing

See [AGENTS.md](AGENTS.md) for guidance on how to contribute to this project.
If you have any feature requests or bug reports, 
please open an issue or feel free to submit a pull request.

## Docs

- [How it works](docs/how-it-works.md)
- [Agents analysis](docs/agents-analysis.md)
- [Release](docs/release.md)
- [Agents](AGENTS.md)
