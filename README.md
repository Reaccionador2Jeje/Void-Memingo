# Void Memingo

Void Memingo is evolving into an operating-system development environment: a Rust application, terminal UI (TUI), and a small scripting language.

## Install

The public Cloudflare Worker currently serves the bootstrap shell script:

```sh
curl -fsSL https://void-memingo-installer.cristopheriguaran-7.workers.dev/install.sh | sh
```

**Important:** the command above still requires curl to retrieve the first installer script. A program cannot download itself from a URL without some network transport. Once you already have the Void executable, its native installer uses Rust HTTPS libraries and does not invoke curl or wget:

```sh
void install
```

The native installer downloads the latest Linux x86_64 GitHub Release, verifies SHA-256, and installs to `~/.local/bin` or `/usr/local/bin`.

A GitHub Release with the expected assets must exist for installation to succeed. A 404 means the release asset URL is not published yet (or the asset filename does not match).

## Terminal interface

Launch the interactive menu:

```sh
void tui
```

Options include project initialization, project checks, configuration display, running scripts, and installing updates.

## Void language (first iteration)

Void scripts use the `.void` extension.

Example `hello.void`:

```text
# A first Void script
let project = "MyOS"
print "Welcome to Void Memingo"
print project
```

Run it:

```sh
void run hello.void
```

Current language features:
- `#` comments
- `let name = value` variable assignment
- `print value` output
- Quoted strings and integer literals

This is an initial interpreter, not yet a kernel or a general-purpose programming language.

## CLI

- `void tui` — open the TUI
- `void install` — download and install the latest release without curl/wget
- `void run <file.void>` — execute a Void script
- `void init [path]` — initialize an OS project
- `void check [path]` — validate project structure and configuration
- `void config [path]` — show project configuration
- `void version` — show version

## Build from source

Requires Rust stable and Cargo:

```sh
cargo test
cargo build --release
```

The executable is `target/release/void`.

## Releases

Push a version tag to trigger the GitHub Actions release workflow:

```sh
git tag v0.2.0
git push origin v0.2.0
```

Assets expected by the installer:
- `void-linux-x86_64`
- `void-linux-x86_64.sha256`

## Cloudflare

The Worker source is in `cloudflare/`. Its public endpoint is:

https://void-memingo-installer.cristopheriguaran-7.workers.dev

The current Worker proxies `install.sh` from the repository. Deploy changes from `cloudflare/` with Wrangler (Node.js 22.12+).

## Current limitations

- Prebuilt release and native installer currently target Linux x86_64.
- The first Void language implementation is intentionally small.
- Kernel, bootloader, compiler toolchain, package manager, and distribution image builder are not implemented yet.

## License

MIT
