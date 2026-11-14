# Void Memingo Desktop

The native desktop application is built with Tauri 2, Rust, React, TypeScript, and Vite. It is separate from the existing \`void\` CLI so the current command-line release remains stable.

## Requirements

- Node.js 20.19+ or 22.12+
- Rust stable
- Tauri 2 system dependencies for your operating system

On Debian/Ubuntu, install the Tauri Linux prerequisites before building. The official Tauri documentation lists the current WebKitGTK, GTK, and appindicator packages.

## Development

From this directory:

\`\`\`sh
npm install
npx tauri dev
\`\`\`

## Production build

\`\`\`sh
npm install
npx tauri build --bundles deb
\`\`\`

Tauri produces Linux `.deb`, Windows `.msi` / `.exe` (NSIS), and macOS `.dmg` bundles. CI builds Linux and Windows x86_64 plus macOS Intel and Apple Silicon variants.

## Workspace security

The Rust backend only reads and writes files within the folder selected by the user. It rejects parent-directory traversal and does not follow symbolic links while listing the workspace. The first milestone includes directory browsing, UTF-8 text editing, creating files, and saving changes.

## Git source control

The Source Control view uses the system Git executable and supports:
- Open an existing local repository or initialize a new one.
- Clone a repository from an HTTPS or SSH Git URL.
- View working-tree status, staged summary, recent commits and local branches.
- Stage all changes, create commits, create/switch branches, add remotes and fetch.
- Run pull and push in the integrated PTY so configured credential helpers and SSH agents can interact.

GitHub, GitLab and other compatible Git hosts work through their standard Git remote URLs. GitVoid is detected as an experimental provider label, but dedicated GitVoid service integration is not available until its remote protocol/API is complete. Credentials remain in the operating system's Git credential helper or SSH agent; do not paste access tokens into repository URLs.

## Integrated terminal

The bottom panel now uses a native cross-platform PTY through `portable-pty` and renders it with xterm.js. Shell sessions stay open between commands, stream output as it arrives, accept interactive input, support terminal resize signals, and can be interrupted with Ctrl+C. Multiple terminal sessions can be opened in tabs and closed independently.

The terminal starts in the selected workspace, but it is **not a security sandbox**: commands run with the current user's operating-system permissions and can access files and tools available to that user outside the workspace. Only run commands you understand and trust. Closing a terminal ends its shell session; child-process cleanup behavior depends on the operating system and shell.

## OS Builder

The OS Builder stores portable distribution settings in `void-os.toml` and keeps OS-specific configuration inside the Void Memingo project. PondOS is an optional example profile, not a special mode or a product dependency.

Current backends:
- Debian Live: generates a live-build workspace, package lists, filesystem overlay, hooks and a reviewable build script.
- Ubuntu Live: generates a live-build/casper profile, package list, overlay, hooks and a build script.
- Arch Linux: stages the upstream archiso releng profile, package list, airootfs overlay and an `mkarchiso` build script.

All three remain previews until a reproducible ISO has been built and boot-tested. The integrated PTY can run a reviewed build script with explicit confirmation; the user can interrupt the foreground command with Ctrl+C. Installer generation, partitioning, automated dependency preflight, integrated build progress/cancel controls, QEMU boot tests and full per-desktop wallpaper/theme configuration are not yet complete.

## Remaining IDE modules

Language diagnostics, project-wide symbol search, language services, debugger integration, and richer build lifecycle management remain planned.
