# Git integration and repository providers

Void Memingo uses the system Git executable as its repository engine. The source-control panel works with local repositories and standards-compliant Git remotes instead of implementing a separate copy of Git.

## Supported providers

| Provider | Integration | Status |
| --- | --- | --- |
| Local Git | init, status, stage, commit, branches, fetch | Implemented in desktop backend |
| GitHub | HTTPS or SSH remote | Works through Git remotes and the user's Git credential helper / SSH agent |
| GitLab | HTTPS or SSH remote | Works through Git remotes and the user's Git credential helper / SSH agent |
| Other Git servers | HTTPS, SSH or git:// remote | Generic Git remote support |
| GitVoid | Generic remote URL only | Experimental; dedicated API is blocked on the GitVoid server/protocol being completed |

The source panel detects GitHub and GitLab remotes from their URLs for display. Detection is descriptive, not an authentication mechanism.

## Credentials and account separation

- Git commands run as the current desktop user. They use the operating system's configured Git credential helper or SSH agent.
- Void Memingo must never ask users to paste provider passwords or personal access tokens into the remote URL field.
- Do not persist Git credentials in the project manifest, repository files, application logs, or Supabase tables.
- GitHub / Google login for Atdev OS Hub is identity for the publishing community. It does not automatically grant Git push permissions. Repository access remains governed by Git's own credential helper or SSH configuration.
- A future provider-specific API integration must use OAuth with least-privilege scopes and secure OS credential storage.

## Current desktop commands

The native Rust backend exposes repository status, initialization, stage-all, commit, remote-add, branch-create, branch-checkout and fetch-all commands. Pull and push are sent to the integrated PTY so Git can interact with configured credential helpers and SSH agents.

The workspace path is canonicalized before every backend operation. Git is invoked with argument arrays, never through a shell string, for native operations.

## GitVoid contract

GitVoid is an independent Atdev Dev Git client / service and is still incomplete. Void Memingo treats it as a generic Git remote when it exposes a normal Git transport URL. A dedicated adapter must wait for a stable documented API covering repository discovery, authentication, refs, objects, fetch and push. Do not pretend GitVoid API support exists before that contract is implemented.
