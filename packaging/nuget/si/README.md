# si for .NET

`si` is the command line interface for [sigit.si](https://sigit.si): sign in, manage repositories,
review pull requests, triage issues, search code, and drive Cloud Sessions from the terminal.

## Install

```sh
dotnet tool install --global SiGit.Cli
```

## Update

```sh
dotnet tool update --global SiGit.Cli
```

## Run

```sh
si auth login
si repo list
```

## Platform support

This .NET tool bundles native `si` binaries for:

- macOS `arm64`, `x64`
- Linux `arm64`, `x64` (glibc)
- Windows `arm64`, `x64`

Because all six binaries ship in one package, the install is larger than a single-platform
download. `cargo install sigit-si-cli` builds only for your machine.

## Other installation methods

- Cargo: `cargo install sigit-si-cli`

## Source

- Repository: <https://github.com/getsigit/si>
- Website: <https://sigit.si>
- Issues: <https://github.com/getsigit/si/issues>

## License

[Apache 2.0](https://github.com/getsigit/si/blob/main/LICENSE)
