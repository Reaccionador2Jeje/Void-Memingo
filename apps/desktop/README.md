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

## Integrated command panel (initial implementation)\n\nThe bottom-panel terminal runs commands through the native Rust backend in the selected workspace directory. It captures standard output, standard error, and the exit code; supports a workspace-confined `cd` command and command history; and limits captured output to 1 MiB per stream. Commands run with the current user's operating-system permissions, so only execute commands you trust. This first version is command-oriented rather than a persistent interactive PTY: long-running interactive programs, terminal UI applications, live streaming, and Ctrl+C cancellation are not supported yet.\n\nGit operations, language services, build orchestration, and the debugger remain planned modules.
