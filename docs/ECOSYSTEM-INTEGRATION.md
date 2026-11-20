# Void Memingo — Ecosystem Integration

Status: planning document for the Rust CLI/TUI and scripting environment.

## Compatibility boundaries
- Publish the supported command set and platform matrix for each release.
- Do not imply the install command supports platforms where it does not.
- Keep release checksums tied to exact release artifacts.
- Keep scripting permissions explicit and avoid accidental host-wide file access.

## Next tasks
1. Add CLI contract tests for version, help and error exit codes.
2. Test release artifact checksums in CI.
3. Document differences between direct downloads and installer support.
4. Add fixtures for the scripting language and TUI command routing.