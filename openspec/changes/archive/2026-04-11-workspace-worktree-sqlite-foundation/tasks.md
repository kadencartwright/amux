## 1. SQLite Control-Plane Foundation

- [x] 1.1 Add SQLite storage support to `amuxd`, including schema initialization for workspaces, managed worktrees, and workspace-scoped sessions.
- [x] 1.2 Replace the file-backed session store with a SQLite-backed store boundary that reads and writes session, workspace, and managed-worktree metadata.
- [x] 1.3 Update daemon startup and configuration wiring so the new control-plane store is used for workspace-aware flows instead of `sessions.json`.

## 2. Workspace And Source-Ref APIs

- [x] 2.1 Add workspace models and daemon endpoints to register and list workspaces with `git` versus `none` classification.
- [x] 2.2 Add git source-ref discovery for git workspaces, including both local branches and remote tracking branches.
- [x] 2.3 Add request validation and error handling for unsupported worktree flows on non-git workspaces.

## 3. Managed Worktree Control Plane

- [x] 3.1 Implement AMUX-managed worktree path generation under the hidden sibling worktree root.
- [x] 3.2 Add daemon endpoints and git integration to create managed worktrees from `source_ref` plus a brand-new `branch_name`.
- [x] 3.3 Persist and list only AMUX-managed worktrees, rejecting duplicate managed branch names within a workspace.

## 4. Workspace-Scoped Session Runtime Integration

- [x] 4.1 Extend the session creation contract and tmux runtime path so new sessions launch with an explicit cwd.
- [x] 4.2 Implement `local` session creation bound to the workspace root and `worktree` session creation bound to a managed worktree path.
- [x] 4.3 Extend session retrieval and listing flows so session metadata includes its kind and workspace or managed-worktree binding while runtime liveness still comes from tmux.

## 5. Shell Workflow Extension

- [x] 5.1 Extend the browser shell state and boot flow to load workspaces and keep session creation scoped to a selected workspace.
- [x] 5.2 Add shell UI for creating local sessions in the workspace root and managed worktrees from a selected source ref plus new branch name.
- [x] 5.3 Surface managed worktrees and session context in the shell so users can start and identify worktree sessions separately from local sessions.

## 6. Verification

- [x] 6.1 Add backend tests covering workspace registration, source-ref discovery, managed worktree creation, duplicate-branch rejection, non-git rejection, and restart persistence.
- [x] 6.2 Add shell tests covering workspace-scoped local session creation, managed worktree creation, and worktree-session launch flows.
- [x] 6.3 Add a manual verification path that exercises both git and non-git workspaces, including local sessions, managed worktree creation from local and remote source refs, and restart visibility.
