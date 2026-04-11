## Context

AMUX already has three important baseline pieces in place: a tmux-backed daemon runtime, a terminal surface contract, and a minimal daemon-served browser shell. What is still missing is the product's intended working context model. Sessions can be created and rendered, but they are not yet anchored to a workspace, cannot distinguish root-checkout work from isolated branch work, and persist only through a file-backed JSON map that will not scale to richer control-plane state.

This change introduces the first durable control-plane model for AMUX. It treats workspaces as the top-level context, adds AMUX-managed worktrees for git workspaces, and makes session creation explicit about whether the session is local to the workspace root or attached to a managed worktree. SQLite is introduced here because the change adds relational state that should remain durable across daemon restarts without expanding the runtime boundary away from tmux.

Constraints:
- The current runtime substrate remains tmux-backed.
- The current shell is the thin browser shell already served under `/app/...`; this change extends that shell rather than redesigning it.
- Git worktree creation must support source refs from either local branches or remote tracking branches.
- A selected remote tracking branch is a starting point only; the resulting worktree always checks out a brand-new branch.
- Only AMUX-managed worktrees are in scope for this chunk.

## Goals / Non-Goals

**Goals:**
- Introduce SQLite as the durable control-plane store for AMUX metadata.
- Add a workspace model that distinguishes git and non-git roots.
- Add AMUX-managed worktrees for git workspaces, created from a chosen source ref and a new branch name.
- Add two session kinds:
  - `local`, whose process cwd is the workspace root
  - `worktree`, whose process cwd is the managed worktree path
- Keep session liveness sourced from tmux while moving control-plane metadata to SQLite.
- Extend the shell just enough to let users create local sessions, create managed worktrees, and start worktree sessions.

**Non-Goals:**
- Worktree deletion.
- Discovery or adoption of unmanaged pre-existing git worktrees.
- Attention signals, timeline/logbook views, auth, speech, or multi-pane dashboard work.
- Replacing tmux with a direct PTY runtime.
- Terminal streaming transport changes.
- Large shell information-architecture changes beyond the new workspace/worktree flows.

## Decisions

1. **SQLite becomes the control-plane persistence layer**
   - Decision: replace the file-backed session store with a SQLite-backed metadata store.
   - SQLite owns durable control-plane metadata: workspaces, managed worktrees, sessions, and their relationships.
   - tmux remains authoritative for runtime liveness and terminal interaction.
   - Rationale: this change introduces relational state that is awkward in JSON but still local and simple enough that a server database would be overkill.
   - Alternatives considered:
     - Keep JSON longer: rejected because workspace/worktree/session relationships and future migration needs make the JSON store a dead end.
     - Use Postgres now: rejected because AMUX remains a local-first daemon and does not yet need a networked database.

2. **Session kind is explicit and first-class**
   - Decision: every new session is either `local` or `worktree`.
   - `local` sessions always launch in the workspace root directory.
   - `worktree` sessions always launch in the managed worktree directory.
   - Rationale: this matches the product model directly and avoids inferring session context from cwd conventions after the fact.
   - Alternatives considered:
     - Infer session kind from cwd path: rejected because it is brittle and makes API/UI behavior opaque.
     - Treat worktrees as a property on only some sessions without a kind field: rejected because it leaves non-git and root-workspace flows underspecified.

3. **Worktrees are durable resources distinct from sessions**
   - Decision: a managed worktree is created and stored independently from sessions, and multiple sessions may target the same managed worktree.
   - Each managed worktree belongs to one workspace and one created branch.
   - Rationale: worktrees are durable code contexts; sessions are runtime instances inside them. Separating them avoids needless worktree churn and keeps the session model cheap.
   - Alternatives considered:
     - Create a new worktree every time a worktree session is created: rejected because it couples runtime lifecycle to repository topology and wastes worktree creation.
     - Restrict each worktree to a single session: rejected because there is no product need for that constraint in this phase.

4. **Only AMUX-managed worktrees participate in the control plane**
   - Decision: the daemon stores and exposes only worktrees it created through AMUX.
   - Existing unmanaged git worktrees are ignored for listing and session targeting in this chunk.
   - Rationale: this keeps the initial model bounded and avoids reconciling ownership, naming, and lifecycle for arbitrary pre-existing worktrees.
   - Alternatives considered:
     - Discover and adopt all git worktrees from day one: rejected because it creates unclear ownership rules and extra UI complexity.

5. **Source refs may be local branches or remote tracking branches, but managed worktrees always use a new branch name**
   - Decision: git worktree creation takes two user-controlled inputs:
     - `source_ref`: an existing local branch or remote tracking branch
     - `branch_name`: a brand-new branch to create and check out in the worktree
   - A remote tracking ref is only a base point; it is never the checked-out branch in the managed worktree.
   - Rationale: this keeps the resulting worktree writable and unambiguous while supporting the full set of source refs users asked for.
   - Alternatives considered:
     - Check out the remote tracking ref directly: rejected because it leads to detached or surprising branch state.
     - Reuse an existing local branch automatically when based on a remote ref: rejected because the user asked for a brand-new branch per managed worktree creation.

6. **AMUX-managed worktree paths live in a hidden sibling tree, not inside the workspace root**
   - Decision: managed worktrees are created under a hidden sibling directory adjacent to the workspace root, using a layout like `<workspace-parent>/.amux-worktrees/<workspace-slug>/<branch-slug>`.
   - Rationale: placing worktrees inside the workspace root would create nested repositories and pollute the main checkout. A hidden sibling location keeps them grouped while staying out of the repo contents.
   - Alternatives considered:
     - Place worktrees inside the workspace root: rejected because it would surface as nested repo noise in the root checkout.
     - Use arbitrary user-provided paths now: rejected because it increases scope and makes lifecycle assumptions harder to validate.

7. **No legacy JSON migration in this chunk**
   - Decision: this change replaces the current file-backed control-plane store without importing old `sessions.json` metadata.
   - Rationale: AMUX is still in a foundational rewrite phase, and introducing compatibility paths for pre-workspace sessions would add complexity to the new model before it is stable.
   - Alternatives considered:
     - Import legacy sessions into SQLite as workspace-unbound records: rejected because it creates a transitional session mode that does not fit the new model.

## Risks / Trade-offs

- [SQLite introduces synchronous local I/O into the control plane] -> Keep the schema small, transactions short, and isolate storage access behind one store boundary that can be optimized later if needed.
- [Git command edge cases can complicate worktree creation] -> Keep the first slice narrow: explicit workspace roots, explicit source refs, explicit new branch names, and AMUX-owned worktree paths only.
- [No unmanaged worktree adoption may frustrate users with existing git setups] -> Make the scope boundary explicit in UI and API responses for this milestone.
- [No legacy JSON import drops prior control-plane metadata] -> Treat this as acceptable during the rewrite phase and call it out in release notes for this change.
- [Sessions now depend on workspace registration before creation] -> Keep the shell flow thin and direct so the extra modeling step does not feel like a full project-management layer.

## Migration Plan

1. Add the SQLite dependency, schema creation, and store boundary in `amuxd`.
2. Add workspace, managed worktree, and extended session metadata tables.
3. Extend daemon APIs to register/list workspaces, list source refs, create/list managed worktrees, and create sessions against workspace-root or managed-worktree context.
4. Extend the tmux runtime launch path to accept a cwd for new sessions.
5. Extend the browser shell to select a workspace, create local sessions, create managed worktrees, and start worktree sessions.
6. Remove reliance on the file-backed `sessions.json` store for the new control-plane flows.

Rollback strategy:
- If the SQLite-backed workspace/worktree flows are unstable, disable or revert the new workspace-aware routes and shell paths while preserving the tmux runtime and terminal surface baseline.

## Open Questions

- Should the shell lead with a single "current workspace" model at first, or allow browsing multiple registered workspaces immediately?
- Do we want to expose branch filtering/search in the first shell pass, or defer that until after the basic source-ref flow is stable?
