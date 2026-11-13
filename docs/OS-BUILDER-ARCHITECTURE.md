# Void Memingo OS Builder

## Product goal

Void Memingo must grow from an operating-system development IDE into a distribution workbench. A user should be able to define a Linux-based operating system, customize its root filesystem and desktop experience, build a bootable ISO, and verify the resulting artifact.

The ISO is the final deliverable of an OS project. The UI must never label a profile, generated configuration, or incomplete build as a finished ISO.

## Core workflow

1. Create or open an OS project.
2. Choose a supported Linux base and release.
3. Choose target architecture and image type.
4. Configure the root filesystem and package set.
5. Choose a desktop environment or a headless/server profile.
6. Customize the desktop session, applications, themes, defaults, services, locale, users, and branding.
7. Review a build summary and dependency checks.
8. Build the ISO with a distro-specific backend.
9. Inspect the build log and validation report; open the output directory.

## Project model

An OS project should be declarative and versionable. The initial project format will be void-os.toml with sections for:

- Project identity: name, identifier, version, publisher, description, ISO label.
- Base: distribution, release, architecture, repositories and mirrors.
- Packages: requested packages, package groups, removals and optional packages.
- Filesystem: root overlay, additional files, directory permissions, ownership, enabled services, locale, timezone, hostname, users, initramfs and kernel choices where supported.
- Storage: target installation filesystem preference (ext4, Btrfs, XFS where supported); this is distinct from the compressed filesystem used inside a live ISO.
- Desktop: desktop environment, display manager, session, default applications, themes, icons, fonts, wallpapers, panel/dock configuration and desktop defaults.
- Boot: UEFI/BIOS support, bootloader and live-session options where supported.
- Output: ISO filename, volume label, checksums and build directory.

Filesystem overlays must be confined to the OS project's own directory. The builder must reject path traversal and must never copy arbitrary host root paths into an image.

## Build backend strategy

Do not force all distributions through one tool. Use a backend interface with explicit capability declarations and separate implementations.

### Supported base distributions

Void Memingo has exactly three official Linux bases:

1. **Debian**
2. **Ubuntu**
3. **Arch Linux**

Do not show Fedora, openSUSE or other distributions as selectable bases unless the product scope is explicitly changed later.

Each base has its own backend implementation and capability table:

- **Debian:** use Debian Live and live-build, with Debian Live configuration, package lists and root filesystem overlays.
- **Ubuntu:** use an Ubuntu-specific build path based on Ubuntu's livecd-rootfs configuration and live-build integration, with casper for live boot behavior. For workflows that customize an existing official Ubuntu installer image, evaluate livefs-editor as a distinct image-editing mode; do not pretend that editing an existing ISO and building a new image from a clean project are identical workflows.
- **Arch Linux:** use archiso profiles, package lists and the airootfs/ overlay.

The backend interface must expose supported releases, architectures, desktop profiles, boot modes, installer capabilities and required host dependencies for each base. The UI must derive its available options from that capability table.

The app must detect whether the relevant builder and required host dependencies are installed before enabling Build. Unsupported combinations must be disabled with an explanation, not silently approximated. A distribution is not considered supported until its backend can create and validate a bootable ISO in an automated or documented test environment.

## Desktop profiles

Desktop choices must be backend capability-driven. The UI should show only environments that can actually be assembled for the selected base/release. Initial profile candidates include GNOME, KDE Plasma, Xfce, LXQt, Cinnamon and MATE, but each base/backend combination must be tested before it is advertised as supported.

A user may also choose a minimal/headless profile. Desktop customization should use package selection and declarative configuration files, not brittle post-install GUI automation.

## Build safety and reliability

- Show the exact commands, target base/release, package changes, overlay paths and output location before starting.
- Never run arbitrary project scripts as root by default.
- Do not invoke sudo or request an administrator password invisibly. If a backend requires elevated privileges, explain why and require an explicit user action in a separate, reviewed step.
- Keep build workspaces isolated from the host filesystem.
- Validate all project paths and reject absolute paths, parent traversal and unsafe symlink escapes.
- Stream build output and expose cancellation.
- Preserve failed build logs and diagnostics.
- Do not overwrite an existing ISO without explicit confirmation.
- Record the builder version, source repositories, selected package set and project revision in a build manifest.
- Generate SHA-256 checksums after successful output creation.
- A build is successful only if the backend exits successfully, the ISO exists, is non-empty, passes basic ISO validation, and its checksum is written.
- Where QEMU is available, provide an optional boot test. A successful file build must not be presented as proof that every machine will boot it.

## User interface

Add an OS Builder workspace separate from the source editor, with these areas:

- **Overview:** project name, base, release, architecture, build status and output.
- **Base system:** distro/release, repositories, kernel and architecture.
- **Filesystem:** package manager view, package lists, root overlay tree, services, locale and storage defaults.
- **Desktop:** desktop profile, display manager, applications and appearance presets.
- **Boot & installer:** live session, boot modes and installer options supported by the selected backend.
- **Build:** dependency check, build plan, live logs, cancellation, artifact validation and checksum.

The layout should follow the existing VS Code-like workbench conventions and must not replace or break the existing editor, workspace security model, CLI or integrated terminal.

## Platform constraints

The desktop application remains available on Windows, Linux and macOS. Local ISO generation is initially supported only on Linux hosts because the first backend tools target Linux environments. On other host platforms, the app may still edit OS projects and inspect build configuration, but must clearly explain that local ISO building is unavailable until a supported Linux build environment is connected.

The first practical development and test host is a supported Linux desktop such as Zorin OS. Required packages and minimum disk/RAM recommendations must be detected and shown before building; do not hard-code a promise that the current machine has enough resources.

## Delivery milestones

1. Project schema, validation and capability model for Debian, Ubuntu and Arch Linux.
2. OS Builder workbench UI and project persistence.
3. Debian Live backend with a minimal bootable ISO.
4. Ubuntu backend using Ubuntu-specific live image configuration and casper boot integration.
5. Archiso backend with a minimal bootable ISO.
6. Shared filesystem overlays, package customization and validated desktop profiles for all three bases.
7. ISO inspection, checksums, cancellation and optional QEMU boot tests.
8. CI validation and release documentation.

## Implementation status

Implemented in the desktop branch:

- Rust data model for the OS project manifest, TOML parsing and validation.
- Explicit capability registry containing only Debian, Ubuntu and Arch Linux.
- OS Builder workbench with base/release/desktop selection, package list, root overlay, services, locale, timezone, hostname, target storage filesystem, ISO output settings, GTK theme, icon theme and wallpaper.
- Project creation and validation through `void-os.toml`.
- **Debian Live profile preparation:** creates an isolated live-build workspace, package lists, removal and system configuration hooks, root filesystem overlay and a reviewable build script.
- **Ubuntu profile preparation:** maps supported releases to Ubuntu codenames, prepares a live-build package list including casper, root filesystem overlay and a build script.
- **Archiso profile preparation:** stages the upstream releng profile, custom package list and airootfs overlay, then generates an `mkarchiso` build script.
- GTK theme and icon defaults are written into `/etc/skel`; wallpaper files can be supplied from inside the project overlay and copied into the image.
- The OS Builder can send a prepared build command to the integrated PTY terminal. The user explicitly confirms before the command is sent; `sudo` prompts in the terminal rather than being invoked invisibly. Terminal output streams live and Ctrl+C can interrupt the foreground build.
- Existing output files are protected from overwrite by the generated scripts. The scripts write SHA-256 checksum files.
- The desktop TypeScript initialization-order issue found by CI was corrected by moving project creation below workspace initialization.

Still requires verification or additional work:

- **No bootable ISO has yet been generated and tested end-to-end for Debian, Ubuntu or Arch on a real Linux host.** The three entries are marked as profile previews, not verified build support.
- Ubuntu's live-build + casper flow needs an actual build and boot test against its supported release repositories; it is not yet certified as equivalent to Ubuntu's official installer image.
- Archiso profile integration needs validation against current Archiso package/profile versions.
- Build completion currently relies on the scripts' exit status and output creation; richer post-build ISO inspection, checksum verification in the app, artifact details and optional QEMU boot testing remain to be completed.
- Host dependency detection, minimum disk/RAM checks, build cancellation controls in the UI, and per-step progress reporting remain pending. The integrated terminal provides live output and Ctrl+C.
- Desktop theme values are currently applied as GTK defaults and a profile metadata file; automatic wallpaper application for every desktop environment and full KDE/GNOME/Xfce theme configuration remain pending.
- The installation filesystem preference is stored in the manifest but is not used because no installer or partitioning workflow is implemented.
- Package repository management, installer generation and multi-architecture image builds remain future work.

The UI must continue to distinguish profile preparation from a verified ISO build. Do not mark a backend as ready until a reproducible ISO build and boot validation have passed.
