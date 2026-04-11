## Why

AMUX now has a usable daemon, terminal surface, and browser shell, but sessions are still detached from the repo context that the product is meant to organize around. Introducing SQLite-backed control-plane state together with workspace and AMUX-managed worktree sessions is the next logical slice because it turns sessions into durable, repo-scoped working contexts before attention, auth, and richer dashboard features arrive.

## What Changes

- Introduce a SQLite-backed control-plane store for durable AMUX metadata instead of the current file-backed session map.
- Add workspace records with git-versus-non-git detection and use workspaces as the top-level context for session creation.
- Add two session kinds:
  - `local` sessions that always start in the workspace root directory
  - `worktree` sessions that always start in an AMUX-managed worktree directory
- For git workspaces, add source-ref discovery across both local branches and remote tracking branches.
- Add AMUX-managed worktree creation from a user-selected source ref plus a brand-new branch name.
- Add shell and daemon flows to create local sessions directly from a workspace and worktree sessions from a managed worktree.
- Exclude unmanaged worktree adoption and worktree deletion from this change.

## Capabilities

### New Capabilities
- `workspace-worktree-sessions`: Register workspaces, discover git branch sources, create AMUX-managed worktrees, and create local or worktree sessions bound to those contexts.

### Modified Capabilities

## Impact

- Affected code:
  - `amuxd` control-plane persistence, API handlers, runtime launch context, and git integration
  - `amuxshell-web` workspace/worktree/session creation flows
- Affected systems:
  - durable metadata moves to SQLite
  - tmux remains the runtime source of session liveness
  - git becomes part of the worktree-management control plane for git workspaces
- Affected APIs:
  - new workspace and worktree endpoints
  - extended session creation semantics to target either workspace-root or managed-worktree context
