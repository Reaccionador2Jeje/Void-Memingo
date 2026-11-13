# Void Memingo Desktop Architecture

## Product boundary

Void Memingo has two deliverables:

- **CLI (void)** — the existing Rust command-line tool and scripting runtime. It remains at the repository root and keeps its own version.
- **Desktop IDE** — a separate Tauri 2 application under apps/desktop/. It uses a web-based workbench UI and a Rust native backend.

The desktop app must not silently change CLI behavior or make the CLI depend on a graphical environment.

## Technology

| Layer | Technology | Responsibility |
|---|---|---|
| Desktop shell | Tauri 2 | Native windows, IPC, packaging |
| Native backend | Rust | Workspace operations and future OS-dev services |
| UI | React + TypeScript | Workbench and reusable interface components |
| Build tooling | Vite | Frontend development and production assets |
| Distribution | GitHub Actions | Native platform builds and release artifacts |

## Current structure

- apps/desktop/src/App.tsx
- apps/desktop/src/main.tsx
- apps/desktop/src/styles.css
- apps/desktop/src-tauri/capabilities/default.json
- apps/desktop/src-tauri/src/commands/mod.rs
- apps/desktop/src-tauri/src/commands/workspace.rs
- apps/desktop/src-tauri/src/main.rs
- apps/desktop/src-tauri/Cargo.toml
- apps/desktop/src-tauri/build.rs
- apps/desktop/src-tauri/tauri.conf.json
- apps/desktop/package.json
- apps/desktop/vite.config.ts

## Native command boundaries

The frontend communicates with Rust through explicit Tauri commands. Workspace commands receive the selected root directory and a relative path; they do not accept arbitrary shell commands.

The workspace backend:
- Canonicalizes the selected root.
- Rejects absolute paths and parent-directory traversal.
- Verifies resolved paths remain inside the workspace.
- Does not follow symbolic links while listing directories.
- Reads UTF-8 text and writes files only inside the selected workspace.

The terminal must be a separate, explicitly designed process service. It must not be implemented by turning arbitrary editor file operations into shell execution.

## Workbench modules

The interface is organized around independent modules:

1. Activity rail — navigation between workbench views.
2. Explorer — workspace tree and file operations.
3. Editor — tabs, dirty state, editing, save, and future language services.
4. Bottom panel — diagnostics, output, and future terminal sessions.
5. Status bar — workspace and editor state.
6. Future services — Git, search, build orchestration, debugger, extensions, and OS toolchain.

A module is only marked available after its backend and UI behavior are implemented and tested.

## Packaging

Void Memingo Desktop is a PC application, not a mobile/Android app. The same source and feature set target Windows, mainstream Linux distributions, and macOS. Native bundles are built on native GitHub Actions runners:

- Linux x86_64: `.deb`.
- Windows x86_64: `.msi` and NSIS `.exe`.
- macOS Intel x86_64: `.dmg`.
- macOS Apple Silicon ARM64: `.dmg`.

The release workflow also preserves the existing CLI binaries and VS Code extension. A platform artifact is considered released only after its runner completes successfully.

## Milestones

- [x] Establish separate desktop app package.
- [x] Add Tauri 2 native shell and React/TypeScript workbench.
- [x] Implement secure workspace browsing, text editing, and saving.
- [x] Add frontend and native-backend CI checks.
- [x] Add native installer build steps to release automation.
- [ ] Validate full Linux, Windows, and macOS builds in GitHub Actions.
- [ ] Add a proper code editor component and language-aware editing.
- [ ] Implement a real integrated terminal with process lifecycle management.
- [ ] Add Git operations, project templates, build profiles, and diagnostics.
- [ ] Integrate the Void language toolchain and OS-development workflows.
