# Void Memingo

Void Memingo is a Rust-based toolkit for building operating-system projects and custom Linux distributions.

## Install (Linux x86_64)

The installer downloads the latest published binary, verifies its SHA-256 checksum, and installs `void` globally when it has permission. Otherwise it installs to `~/.local/bin`.

After the Cloudflare Worker is deployed, use:

```sh
curl -fsSL https://YOUR-WORKER.YOUR-SUBDOMAIN.workers.dev/install.sh | sh
```

The installer requires `curl` and `sha256sum`. A published GitHub Release is required before installation can succeed.

## Publish a release

Push a version tag to trigger the GitHub Actions release workflow:

```sh
git tag v0.1.0
git push origin v0.1.0
```

GitHub Actions builds the Linux x86_64 executable and publishes it with a SHA-256 checksum.

## Deploy the Cloudflare installer endpoint

1. Install Node.js/npm and authenticate Wrangler with your Cloudflare account.
2. From this repository, run:

```sh
cd cloudflare
npx wrangler@latest login
npx wrangler@latest deploy
```

3. Cloudflare prints the public `workers.dev` URL. Replace the placeholder in the install command above with that URL.

The Worker serves the installer script from this repository over HTTPS. It does not store credentials or binaries.

## Build from source

Requires Rust stable:

```sh
cargo build --release
cargo test
```

## Current support

- Installer: Linux x86_64
- Release binaries: Linux x86_64
- Other operating systems and architectures are not supported by the installer yet.
