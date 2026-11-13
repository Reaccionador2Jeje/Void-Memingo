# Void Memingo

Void Memingo is evolving into an operating-system development environment: a Rust application, terminal UI (TUI), and a small scripting language.

## Desktop IDE

A native cross-platform desktop workspace is being developed in \`apps/desktop/\` using Tauri 2, Rust, React, TypeScript, and Vite. It is a separate application and does not replace or break the existing \`void\` CLI.

The first desktop foundation provides:
- Native folder selection and workspace navigation.
- A file explorer with lazy directory loading.
- Multi-tab UTF-8 text editing and save support (\`Ctrl+S\` / \`Cmd+S\`).
- A Rust filesystem backend that blocks path traversal and avoids following symbolic links.
- A modular VS Code-inspired workbench layout.

The integrated terminal, Git operations, language server, debugger, build system, and OS-development tooling are future modules. Their panels are visibly marked as not implemented rather than pretending to work.

### Run the desktop application

Requires Node.js 20.19+ or 22.12+, Rust stable, and the platform-specific Tauri 2 prerequisites.

\`\`\`sh
cd apps/desktop
npm install
npx tauri dev
\`\`\`

Build a Debian package on Linux:

\`\`\`sh
npx tauri build --bundles deb
\`\`\`

Release automation is being extended to publish Linux \`.deb\` / \`.rpm\` / AppImage, Windows \`.msi\` / \`.exe\`, and macOS \`.dmg\` installers. Actual release availability depends on successful platform CI builds.

## Supported platforms

Release builds are provided for Linux x86_64, Windows x86_64, macOS Intel (x86_64), and macOS Apple Silicon (ARM64).

Download the matching executable and SHA-256 file from the [GitHub Releases](https://github.com/Reaccionador2Jeje/Void-Memingo/releases) page.

On Linux and macOS:
```sh
chmod +x ./void
./void version
```

On Windows PowerShell:
```powershell
.\void-windows-x86_64.exe version
```

The built-in `void install` command currently installs the Linux x86_64 release only. Windows and macOS binaries are available as direct downloads; native installers for those platforms are planned separately.

## Visual Studio Code

A starter extension is included in `vscode-extension/`. It provides .void file recognition, syntax highlighting, snippets, and a **Void Memingo: Run Current File** command that runs the active file using the `void` executable on PATH.

To package it locally:
1. Install Node.js 22 or newer.
2. Open a terminal in `vscode-extension/`.
3. Run `npm install --global @vscode/vsce`.
4. Run `vsce package`.
5. In VS Code, choose **Install from VSIX...** and select the generated `.vsix`.

The release workflow also attaches `void-memingo-vscode.vsix` to each GitHub Release.

## Linux native installer

The public Cloudflare Worker currently serves the bootstrap shell script:

```sh
curl -fsSL https://void-memingo-installer.cristopheriguaran-7.workers.dev/install.sh | sh
```

The command above requires curl for the first download. Once installed, the native installer uses Rust HTTPS libraries and does not invoke curl or wget. It downloads the latest Linux x86_64 release, verifies SHA-256, and installs to `~/.local/bin` or `/usr/local/bin`.

## Terminal interface

Run `void tui` to open the interactive menu.

## Void language

Void scripts use the `.void` extension.

Example `hello.void`:
```text
# A first Void script
let project = "MyOS"
print "Welcome to Void Memingo"
print project
```

Run it with:
```sh
void run hello.void
```

Current language features: `#` comments, `let name = value`, `print value`, quoted strings, and integer literals. This is an initial interpreter, not yet a kernel or general-purpose programming language.

## CLI

- `void tui` — open the TUI
- `void install` — install/update the latest Linux x86_64 release
- `void run <file.void>` — execute a script
- `void init [path]` — initialize an OS project
- `void check [path]` — validate a project
- `void config [path]` — display project configuration
- `void version` — show version

## Build from source

Requires Rust stable and Cargo:
```sh
cargo test
cargo build --release
```

## Releases

Pushing a version tag triggers GitHub Actions to build platform binaries and package the VS Code extension.

## Cloudflare

The Worker source is in `cloudflare/`. Its public endpoint is https://void-memingo-installer.cristopheriguaran-7.workers.dev. Deploy changes from `cloudflare/` with Wrangler (Node.js 22.12+).

## Current limitations

- The built-in native updater currently supports Linux x86_64 only; Windows and macOS use direct downloads.
- The VS Code extension provides basic language support and runs the installed CLI; it is not a full IDE or debugger yet.
- The Void language, kernel, bootloader, compiler toolchain, package manager, and distribution image builder are still early-stage or not implemented.

## License

MIT
