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
