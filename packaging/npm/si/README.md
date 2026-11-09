<!-- mcp-name: si.sigit/cli -->

# si for npm

`si` is the command line interface for [sigit.si](https://sigit.si): sign in, manage repositories,
review pull requests, triage issues, search code, and drive Cloud Sessions from the terminal.

This package installs the native `si` binary for your platform. It has no JavaScript API.

## Install

```sh
npm install -g @getsigit/si
```

Or run it without installing:

```sh
npx @getsigit/si --help
```

## Run

```sh
si auth login
si repo list
```

## Platform support

The binary comes from an optional dependency that npm selects for your machine:

- macOS `arm64`, `x64`
- Linux `arm64`, `x64` (glibc)
- Windows `arm64`, `x64`

Installing with `--omit=optional` leaves the binary out, and `si` then fails with a message saying so.

## Other installation methods

- Cargo: `cargo install sigit-si-cli`
- PyPI: `pipx install sigit-si-cli`
- .NET: `dotnet tool install --global SiGit.Cli`
- Prebuilt binaries: [GitHub Releases](https://github.com/getsigit/si/releases)

## Source

- Repository: <https://github.com/getsigit/si>
- Website: <https://sigit.si>
- Issues: <https://github.com/getsigit/si/issues>

## License

[Apache 2.0](https://github.com/getsigit/si/blob/main/LICENSE)
