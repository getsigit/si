# sigit-si-cli

`si` is the command line interface for [sigit.si](https://sigit.si) — sign in,
manage repositories, review pull requests, triage issues, search code, and
drive Cloud Sessions, all from the terminal. It's the `gh` of sigit.si.

## Install

```sh
cargo install --path crates/cli
```

This installs a binary named `si`.

## Usage

```sh
si auth login                     # sign in with your sigit.si account
si repo list                      # your repositories
si repo create my-project --private

# Inside a repo checkout, si infers OWNER/NAME from the sigit.si git remote —
# --repo is only needed when you're not standing in one.
si pr list
si pr view 42
si pr create --title "Fix the thing" --base main

si issue list --state open
si issue create --title "Bug: ..." --body "Steps to reproduce..."

si code search "TODO" --ref main
si code view README.md

si session new --title "Refactor auth"
si billing show

si browse                          # open the current repo on sigit.si
si api repos                       # raw escape hatch for any endpoint
```

Pass `--json` to any read command for machine-readable output.

### Environments

```sh
si --environment dev repo list          # a local sigit.si server (localhost:3000)
SIGIT_HOST=staging.example.com si repo list
```

Credentials are stored per-environment in `~/.sigit/token` (or
`~/.sigit-dev/token`). `SIGIT_TOKEN` overrides the stored token, which is how
CI authenticates without an interactive login.

## Workspace layout

- **`crates/sigit-api`** — a standalone, typed Rust client for the sigit.si
  JSON API. No CLI dependencies (clap, dialoguer, …); usable from any Rust
  project that wants to talk to sigit.si, including the desktop app.
- **`crates/cli`** — the `si` binary: command parsing, terminal rendering, git
  remote resolution, and credential storage, built on top of `sigit-api`.

Issues, pull requests, and code search have no REST surface on sigit.si yet —
they're served over the same MCP (Model Context Protocol) JSON-RPC endpoint
the AI agent uses. `sigit-api` speaks that transport internally; it's not
visible at the call site.

## Development

```sh
make build   # cargo build --workspace
make test    # cargo test --workspace
make lint    # cargo clippy --workspace --all-targets -- -D warnings
make fmt     # cargo fmt --all
make check   # fmt + lint + test
```

## License

Apache-2.0
