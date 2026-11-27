# si for Python

`si` is the command line interface for [sigit.si](https://sigit.si): sign in, manage repositories,
review pull requests, triage issues, search code, and drive Cloud Sessions from the terminal.

This package is the native `si` binary packaged as a Python wheel. There is no Python API.

## Install

```sh
pipx install sigit-si-cli
```

Or run it without installing:

```sh
uvx --from sigit-si-cli si --help
```

## Run

```sh
si auth login
si repo list
```

## Platform support

Prebuilt wheels cover:

- macOS `arm64`, `x86_64`
- Linux `arm64`, `x86_64` (glibc)
- Windows `x86_64`

On other platforms pip builds from the source distribution, which needs a Rust toolchain.

## Other installation methods

- Cargo: `cargo install sigit-si-cli`
- npm: `npm install -g @getsigit/si`
- .NET: `dotnet tool install --global SiGit.Cli`
- Prebuilt binaries: [GitHub Releases](https://github.com/getsigit/si/releases)

## Source

- Repository: <https://github.com/getsigit/si>
- Website: <https://sigit.si>
- Issues: <https://github.com/getsigit/si/issues>

## License

[Apache 2.0](https://github.com/getsigit/si/blob/main/LICENSE)
