# Atdev OS Hub

Atdev OS Hub is the community and publishing service for operating-system projects created with Void Memingo. It is designed as a focused OS catalogue and collaboration network, not a replacement for GitHub or GitLab.

## Product goals

- Publish operating-system project pages, releases, screenshots, documentation and verified checksums.
- Discover distributions by base, architecture, desktop environment, release channel and tags.
- Follow maintainers, star projects, comment on releases and report issues.
- Link each published project to its canonical source repository and build provenance.
- Allow users to publish PondOS and future Atdev Dev operating systems without forcing their source code to be hosted by Atdev.

## Identity

Supabase Auth provides sign-in with:
- GitHub
- Google

The two identity providers are account-login options. The user can link identities to one profile after authenticating both providers. Never merge two accounts solely because their email strings match; require an authenticated linking flow.

GitLab remains supported as a source-code remote in Void Memingo. It is not required as an OS Hub login provider for the first milestone.

## Repository and artifact model

A published OS record contains:
- owner profile and project slug
- name, summary, description, license and tags
- base distribution, release, architectures and desktop environments
- source provider and repository URL
- release records with version, channel, publication time, artifact URL, file size and SHA-256
- screenshots, documentation links and build provenance

The first milestone stores catalogue metadata and small images in Supabase. ISO binaries remain in GitHub Releases or another user-selected artifact host; OS Hub stores their URLs, sizes and SHA-256 values. This avoids making a database-backed service the default host for multi-gigabyte images.

## Security rules

- Use a publishable Supabase key in the browser. Never ship a service-role key or OAuth client secret.
- Enable RLS on every exposed table.
- Public users may read published projects and public release metadata.
- Only authenticated owners may create or update their own projects and releases.
- Comments and stars require authentication; owners may edit/delete their own comments.
- Store profile roles in trusted app metadata or a server-controlled table, never in user-editable JWT metadata.
- Private drafts must not be readable by anonymous users.
- Uploads must be size-limited and restricted to image MIME types for the initial screenshot bucket.
- Validate artifact URLs and checksums server-side before displaying a release as verified.

## Supabase project setup

The currently connected Supabase project named `Atdev OpenWeb Linux` is reported as INACTIVE. No schema changes or project restoration have been performed. Before deployment:
1. Choose or restore an active Supabase project in the Atdev Dev organization.
2. Configure GitHub and Google OAuth credentials in Supabase Auth.
3. Set the project URL and publishable key as frontend environment variables.
4. Apply the SQL migration in `supabase/migrations/`.
5. Create the public screenshot bucket with upload restrictions.
6. Configure allowed redirect URLs for the production and local development origins.

## Planned milestones

1. Public catalogue, project pages, Google/GitHub sign-in and owner-managed project metadata.
2. Release publishing, screenshot upload, SHA-256 verification and download statistics.
3. Comments, stars, follows and moderation/reporting.
4. Void Memingo publishing workflow: prepare build metadata, attach source revision, publish release entry and link the ISO artifact.
5. Optional GitHub / GitLab repository import and CI build provenance.

## Relationship with PondOS

PondOS is the first showcase project: Ubuntu / Ubuntu Server base, LXQt desktop, natural green-and-blue visual identity, pond-and-frog theme, and the minimalist green frog-head-and-leaf logo. Its project page should include release notes, screenshots, documentation, source repository and ISO checksums.
