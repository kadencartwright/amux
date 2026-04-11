## ADDED Requirements

### Requirement: Workspace registration and classification
The system SHALL persist registered workspaces and classify each workspace as either `git` or `none` based on the workspace root.

#### Scenario: Register git workspace
- **WHEN** a user registers a workspace whose root path is a git repository
- **THEN** the system stores the workspace as durable control-plane metadata
- **AND** the workspace is classified as `git`

#### Scenario: Register non-git workspace
- **WHEN** a user registers a workspace whose root path is not a git repository
- **THEN** the system stores the workspace as durable control-plane metadata
- **AND** the workspace is classified as `none`

### Requirement: Source-ref discovery for git workspaces
The system SHALL expose worktree source refs for git workspaces from both local branches and remote tracking branches.

#### Scenario: List local and remote tracking refs
- **WHEN** a user requests worktree source refs for a git workspace
- **THEN** the returned source-ref set includes local branches and remote tracking branches

#### Scenario: Non-git workspace has no worktree source refs
- **WHEN** a user requests worktree source refs for a non-git workspace
- **THEN** the system returns no worktree source refs for that workspace

### Requirement: AMUX-managed worktree creation
The system SHALL create AMUX-managed worktrees for git workspaces from a selected source ref and a brand-new branch name.

#### Scenario: Create managed worktree from local branch
- **WHEN** a user creates a managed worktree for a git workspace using a local branch as `source_ref` and a new `branch_name`
- **THEN** the system creates a managed worktree record
- **AND** the resulting worktree checks out the new `branch_name`
- **AND** the new branch is based on the selected local branch

#### Scenario: Create managed worktree from remote tracking branch
- **WHEN** a user creates a managed worktree for a git workspace using a remote tracking branch as `source_ref` and a new `branch_name`
- **THEN** the system creates a managed worktree record
- **AND** the resulting worktree checks out the new `branch_name`
- **AND** the new branch is based on the selected remote tracking branch
- **AND** the system does not treat the selected remote tracking branch itself as the checked-out branch of the managed worktree

#### Scenario: Reject duplicate managed branch name
- **WHEN** a user attempts to create a managed worktree whose `branch_name` is already used by an AMUX-managed worktree in the same workspace
- **THEN** the system rejects the request

### Requirement: Managed worktree ownership boundary
The system SHALL expose only AMUX-managed worktrees through this capability.

#### Scenario: Unmanaged git worktrees are not surfaced
- **WHEN** a git repository contains worktrees that were not created through AMUX
- **THEN** the system does not expose those unmanaged worktrees as managed worktree records

### Requirement: Workspace-scoped session kinds
The system SHALL create sessions within a workspace as either `local` sessions or `worktree` sessions.

#### Scenario: Local session launches in workspace root
- **WHEN** a user creates a `local` session for a workspace
- **THEN** the session is associated with that workspace
- **AND** the session process launches with the workspace root as its cwd

#### Scenario: Worktree session launches in managed worktree path
- **WHEN** a user creates a `worktree` session for a managed worktree
- **THEN** the session is associated with that managed worktree and its parent workspace
- **AND** the session process launches with the managed worktree path as its cwd

#### Scenario: Non-git workspace rejects worktree session creation
- **WHEN** a user attempts to create a `worktree` session for a workspace classified as `none`
- **THEN** the system rejects the request

### Requirement: Durable SQLite-backed control-plane metadata
The system SHALL persist workspace, managed-worktree, and workspace-scoped session metadata in SQLite so that control-plane relationships survive daemon restart.

#### Scenario: Workspace and managed worktree remain visible after restart
- **WHEN** a workspace and an AMUX-managed worktree have been created and the daemon restarts
- **THEN** the workspace and managed worktree metadata remain available after readiness is restored

#### Scenario: Session bindings remain durable across restart
- **WHEN** a workspace-scoped session has been created and the daemon restarts
- **THEN** the session metadata still identifies its session kind and workspace or managed-worktree binding after readiness is restored
