# Void Memingo

**Void Memingo** is a Rust-based toolkit for building operating-system projects and custom Linux distributions.

## Quick install

**Supported platform:** Linux x86_64.

Install the latest published version with:

```sh
curl -fsSL https://void-memingo-installer.cristopheriguaran-7.workers.dev/install.sh | sh
```

The installer:
- Downloads the latest Linux x86_64 binary from GitHub Releases.
- Verifies its SHA-256 checksum before installation.
- Installs to `/usr/local/bin/void` when it has permission.
- Falls back to `~/.local/bin/void` when system-wide installation is unavailable.

Requirements: `curl` and `sha256sum`. If the installer uses `~/.local/bin`, add that directory to your `PATH` if it is not already there:

```sh
export PATH="$HOME/.local/bin:$PATH"
```

Check the installation:

```sh
void --help
void version
```

> The installer endpoint is live, but installation requires at least one published GitHub Release containing the binary and checksum.

## Commands

| Command | Description |
| --- | --- |
| `void help` or `void --help` | Show command help |
| `void version` | Show the installed version |
| `void init [path]` | Initialize a project |
| `void check [path]` | Validate project configuration and structure |
| `void config [path]` | Display project configuration |

Initialize and inspect a project:

```sh
void init my-operating-system
cd my-operating-system
void check
void config
```

Void creates a `void.toml` file and the initial project directories for the compiler, operating system, distribution, tools, runtime, examples, tests, and documentation.

## Project configuration

Example `void.toml`:

```toml
[project]
name = "MyOperatingSystem"
version = "0.1.0"

[build]
target = "x86_64-unknown-none"
```

The build target defaults to `x86_64-unknown-none` when the `[build]` section is omitted.

## Build from source

Requirements:
- Rust stable toolchain
- Cargo

Build and run the tests:

```sh
cargo test
cargo build --release
```

The executable is created at `target/release/void`.

## Releases

Releases are built automatically by GitHub Actions when a version tag is pushed. The workflow runs the test suite, builds the release executable, generates a SHA-256 checksum, and attaches both files to the GitHub Release.

For the next release:

```sh
git tag v0.1.0
git push origin v0.1.0
```

Release assets:

- `void-linux-x86_64`
- `void-linux-x86_64.sha256`

Do not create the tag again if it already exists. Check the repository's Releases page first.

## Cloudflare installer

The public installer endpoint is hosted on Cloudflare Workers:

- Installer script: https://void-memingo-installer.cristopheriguaran-7.workers.dev/install.sh
- Worker root: https://void-memingo-installer.cristopheriguaran-7.workers.dev

Worker source and configuration are in `cloudflare/`.

To deploy updates, use Node.js **22.12.0 or newer** and run:

```sh
cd cloudflare
npx wrangler@latest login
npx wrangler@latest deploy
```

The Worker fetches `install.sh` from this repository's `main` branch and caches the response briefly. It does not store release binaries or Cloudflare credentials.

## Current support

| Component | Support |
| --- | --- |
| CLI | Linux and other platforms where the Rust binary can run; release installer currently targets Linux x86_64 |
| Release binary | Linux x86_64 |
| Installer | Linux x86_64 |
| Distribution build system | In development |

## Project status

Void Memingo is an early-stage project. The current version provides project initialization, TOML configuration loading and validation, a CLI, and the automated Linux x86_64 release pipeline. Kernel compilation, bootloader integration, package management, and full distribution generation are future development areas.

## License

MIT
