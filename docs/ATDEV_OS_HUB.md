# Atdev OS Hub publishing from Void Memingo

Void Memingo OS Builder can publish a project manifest to Atdev OS Hub. The first integration milestone synchronizes project metadata; build provenance is supported by the server endpoint and will be wired to the build completion lifecycle in a later step.

## One-time setup

1. Sign in to Atdev OS Hub.
2. Open Git Center → Void Memingo · Acceso API.
3. Create a token. It is shown only once and expires after one year.
4. Copy the token into a password manager or another safe local place. Never commit it.
5. Copy the Supabase publishable key from the Atdev OS Hub project's API settings. It is public and is not a service-role/secret key.

## Publish from the OS Builder

1. Open a Void Memingo OS project and configure its name, ID, version, base and desktop.
2. Select **Publicar en OS Hub**.
3. Paste the Supabase publishable key and the personal access token when prompted.
4. Void Memingo submits a normalized manifest to the authenticated sync endpoint.
5. Atdev OS Hub creates or updates the user's OS project and records the Void Memingo external project ID.

The token and publishable key are used only in memory for that request. Void Memingo does not save either value in `void-os.toml` or the Git repository.

## API contract

Endpoint:

```text
https://pzfvpvioxpcjkxdawnsq.supabase.co/functions/v1/void-memingo-sync
```

Headers:
- `apikey: <Supabase publishable key>`
- `Authorization: Bearer <Atdev OS Hub personal access token>`
- `Content-Type: application/json`

Project manifest action: `publish_project`.

The API token is stored in Supabase only as a SHA-256 hash and has `os:publish` and `build:submit` scopes. Revocation is available from Git Center.

## Build provenance

The server also accepts `build_event` requests with the external project ID, version, status, source commit, artifact HTTPS URL and SHA-256. It records build history but does not treat a client-submitted checksum as server-side artifact verification. A build artifact is not automatically promoted to a public release; publishing a release remains a separate explicit action.

## Security

- Never use the Supabase secret/service-role key in Void Memingo.
- Never put personal access tokens in `void-os.toml`, source code, screenshots, terminal logs or Git.
- Revoke a token from Git Center if it is exposed.
- Tokens are scoped and expire; each token is associated with one OS Hub user.
