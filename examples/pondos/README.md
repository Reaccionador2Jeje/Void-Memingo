# PondOS project template

This is the first Void Memingo manifest for PondOS.

## Identity

- Ubuntu / Ubuntu Server family base
- LXQt desktop
- Natural green and blue palette inspired by a pond
- Minimal green frog-head logo with a leaf
- Hostname: `pondos`
- Spanish (Chile) locale and Santiago timezone defaults

## Filesystem overlay

Create the following project overlay before building:

```text
filesystem/
└── usr/
    └── share/
        └── backgrounds/
            └── pondos/
                └── pondos-wallpaper.png
```

Add the PondOS wallpaper image at that path. Add actual GTK theme assets and any desktop-environment-specific configuration to the overlay as they are created. The manifest records the intended theme names; it does not manufacture theme artwork.

## Build status

The manifest is a starting configuration, not a certified installer image. The Ubuntu backend is still a profile preview and requires a successful build and boot test. The current manifest leaves the installer disabled; Calamares / installation workflow must be implemented and tested separately before it is advertised as installable.


## Void Memingo workflow

Open this directory as a workspace in Void Memingo. Keep `pondos.toml` as the source of truth for the OS Builder and commit the complete project (manifest, filesystem overlay, artwork, scripts and documentation) to Git.

Recommended repository layout:

```text
pondos/
├── pondos.toml
├── README.md
├── filesystem/
│   └── usr/share/backgrounds/pondos/
├── scripts/
└── dist/
```

The `dist/` directory contains generated ISO artifacts and should normally be excluded from Git; publish those files as release assets and record their URL and SHA-256 in Atdev OS Hub. Use a GitHub or GitLab remote for source hosting. GitVoid can be selected when its standard Git transport is available; until then, keep it marked experimental and use a normal Git remote.
