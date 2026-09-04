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

### As an MCP server

<!-- mcp-name: si.sigit/cli -->

`si mcp` serves these same commands to an AI agent as
[MCP](https://modelcontextprotocol.io) tools over stdio:

```jsonc
{
  "mcpServers": {
    "sigit": { "command": "si", "args": ["mcp"] }
  }
}
```

There is nothing to authenticate — the server reuses the token `si auth login`
stored. Point your editor at the directory of a checkout and repository tools
default to the repo you're standing in, so the agent can say "list the open
pull requests" without naming one; `create_pull_request` likewise defaults its
head to the checked-out branch.

Trim the tool list when an agent doesn't need all of it:

```sh
si mcp --read-only                    # drop every tool that writes
si mcp --toolsets repo,issue,pr,code  # or SIGIT_TOOLSETS=repo,issue
```

Toolsets are `account`, `repo`, `issue`, `pr`, `code`, `hook`, `session`, and
`billing`. sigit.si also hosts a remote MCP server at `/api/v1/mcp` for clients
that would rather connect over HTTP than spawn a binary.

### Environments

```sh
si --environment dev repo list          # a local sigit.si server (localhost:3000)
SIGIT_HOST=staging.example.com si repo list
```

Credentials are stored per-environment in `~/.sigit/token` (or
`~/.sigit-dev/token`). `SIGIT_TOKEN` overrides the stored token, which is how
CI authenticates without an interactive login.

## Workspace layout

- **`crates/sigit-si-api`** — a standalone, typed Rust client for the sigit.si
  JSON API. No CLI dependencies (clap, dialoguer, …); usable from any Rust
  project that wants to talk to sigit.si, including the desktop app.
- **`crates/cli`** — the `si` binary: command parsing, terminal rendering, git
  remote resolution, and credential storage, built on top of `sigit-si-api`.

Issues, pull requests, and code search have no REST surface on sigit.si yet —
they're served over the same MCP (Model Context Protocol) JSON-RPC endpoint
the AI agent uses. `sigit-si-api` speaks that transport internally; it's not
visible at the call site.

## MCP Registry listings

This repo publishes two entries to the
[official MCP Registry](https://registry.modelcontextprotocol.io), one per way
of reaching the same tools, so registry-aware clients can add either in one
click:

| Listing | File | What it is |
| --- | --- | --- |
| `si.sigit/cli` | `server.json` | This CLI: a cargo package listing that runs `si mcp` over stdio. |
| `si.sigit/sigit` | `server-sigit.json` | The hosted server at `sigit.si/api/v1/mcp`, a remote Streamable-HTTP listing. The endpoint `sigit-si-api` talks to, and the official server siGit Code bakes in. |

The CLI holds `server.json` because it's what this repo ships, and that's the
filename `mcp-publisher` defaults to. Sibling listings are named after their
own leaf, so the file says which entry it publishes.

Both go out through the `release-mcp-registry.yml` workflow, and both prove
namespace ownership with a DNS TXT record on `sigit.si` rather than GitHub
OIDC, so one credential covers the pair. A package listing has a second hurdle:
the registry fetches the crate from crates.io and looks for the
`mcp-name: si.sigit/cli` marker in its README, which is why that marker sits in
this file. Crate versions are immutable, so a release that ships without the
marker can never be listed, and fixing it costs a new version. The internal
setup and release runbook live with the server (private `sigit-si` repo).

## Development

```sh
make build   # cargo build --workspace
make test    # cargo test --workspace
make lint    # cargo clippy --workspace --all-targets -- -D warnings
make fmt     # cargo fmt --all
make check   # fmt + lint + test
```

## Disclaimer

All em dashes are written by Seto Elkahfi.

## License

Apache-2.0
