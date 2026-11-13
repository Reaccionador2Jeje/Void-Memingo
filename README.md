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

The integrated terminal, native Git source-control panel, OS Builder, and initial Atdev OS Hub publishing integration are now part of the desktop foundation. See [Atdev OS Hub integration](docs/ATDEV_OS_HUB.md). The Git panel supports local repositories, branches, commits, remotes, cloning, and fetch; GitHub and GitLab work through standard Git HTTPS/SSH remotes and the user's system credential helper. GitVoid is treated as experimental until its remote service protocol is ready.

Void Memingo Desktop targets **PC operating systems**: Windows 10/11, mainstream Linux distributions, and macOS. It is not an Android application.

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

## OS development and publishing

Void Memingo includes an OS Builder for Debian, Ubuntu, and Arch Linux profile preparation, filesystem overlays, package customization, desktop selection, and local build-script execution. These backends remain profile previews until reproducible ISO builds and boot tests have passed.

[PondOS](examples/pondos/README.md) is the first showcase project: an Ubuntu-based system with LXQt, a green/blue pond-inspired identity, and a minimalist frog-and-leaf logo.

### Git provider support

The desktop source-control panel uses the system Git engine. It supports local Git repositories and standard Git remotes hosted on GitHub, GitLab, or other Git-compatible servers. Credentials stay in the user's OS credential helper or SSH agent. GitVoid is an experimental remote target pending a stable server/API contract.

### Atdev OS Hub

A separate web app is being developed at `apps/os-hub/` for discovering and publishing operating systems. It includes the initial catalogue, GitHub/Google OAuth entry points, and project metadata publishing. Supabase is the planned Auth and database backend; deployment remains blocked until an active project and OAuth provider credentials are configured.

Run the OS Hub locally:
```sh
cd apps/os-hub
cp .env.example .env.local
# Set the Supabase URL and publishable key in .env.local
npm install
npm run dev
```

Review `docs/GIT-INTEGRATION.md`, `docs/ATDEV-OS-HUB.md`, and `supabase/schema-draft.sql` before connecting production services.

## Supported platforms

Desktop release targets are Linux x86_64, Windows x86_64, macOS Intel (x86_64), and macOS Apple Silicon (ARM64). The same desktop codebase and feature set are shared across these PC platforms; installers are generated on native runners.

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
