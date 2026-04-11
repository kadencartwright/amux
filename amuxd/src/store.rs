use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, Error as SqlError, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{now_rfc3339, AppError};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceKind {
    Git,
    None,
}

impl WorkspaceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Git => "git",
            Self::None => "none",
        }
    }

    fn from_str(value: &str) -> Result<Self, AppError> {
        match value {
            "git" => Ok(Self::Git),
            "none" => Ok(Self::None),
            other => Err(AppError::Runtime(format!(
                "unexpected workspace kind in store: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    Local,
    Worktree,
}

impl SessionKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Worktree => "worktree",
        }
    }

    fn from_str(value: &str) -> Result<Self, AppError> {
        match value {
            "local" => Ok(Self::Local),
            "worktree" => Ok(Self::Worktree),
            other => Err(AppError::Runtime(format!(
                "unexpected session kind in store: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceRefKind {
    LocalBranch,
    RemoteTrackingBranch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub kind: WorkspaceKind,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedWorktree {
    pub id: String,
    pub workspace_id: String,
    pub branch_name: String,
    pub source_ref: String,
    pub path: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceRef {
    pub name: String,
    pub kind: SourceRefKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSession {
    pub id: String,
    pub name: String,
    pub runtime_name: String,
    pub kind: SessionKind,
    pub workspace_id: String,
    pub managed_worktree_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct NewWorkspace {
    pub name: String,
    pub root_path: String,
    pub kind: WorkspaceKind,
}

#[derive(Debug, Clone)]
pub struct NewManagedWorktree {
    pub workspace_id: String,
    pub branch_name: String,
    pub source_ref: String,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct NewStoredSession {
    pub id: String,
    pub name: String,
    pub runtime_name: String,
    pub kind: SessionKind,
    pub workspace_id: String,
    pub managed_worktree_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ControlStore {
    path: PathBuf,
}

impl ControlStore {
    pub fn load(path: PathBuf) -> Result<Self, AppError> {
        let store = Self { path };
        let connection = store.open()?;
        store.init_schema(&connection)?;
        Ok(store)
    }

    pub fn insert_workspace(&self, input: NewWorkspace) -> Result<Workspace, AppError> {
        let workspace = Workspace {
            id: Uuid::new_v4().to_string(),
            name: input.name,
            root_path: input.root_path,
            kind: input.kind,
            created_at: now_rfc3339(),
        };

        let connection = self.open()?;
        self.init_schema(&connection)?;
        let result = connection.execute(
            "INSERT INTO workspaces (id, name, root_path, kind, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                workspace.id,
                workspace.name,
                workspace.root_path,
                workspace.kind.as_str(),
                workspace.created_at,
            ],
        );
        match result {
            Ok(_) => Ok(workspace),
            Err(error) if is_unique_violation(&error) => Err(AppError::conflict(
                "workspace_exists",
                format!("workspace already registered: {}", workspace.root_path),
            )),
            Err(error) => Err(AppError::Runtime(format!(
                "failed inserting workspace: {error}"
            ))),
        }
    }

    pub fn list_workspaces(&self) -> Result<Vec<Workspace>, AppError> {
        let connection = self.open()?;
        self.init_schema(&connection)?;
        let mut statement = connection
            .prepare(
                "SELECT id, name, root_path, kind, created_at FROM workspaces ORDER BY created_at DESC, name ASC",
            )
            .map_err(|error| AppError::Runtime(format!("failed preparing workspace list query: {error}")))?;
        let rows = statement
            .query_map([], |row| {
                Ok(Workspace {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    root_path: row.get(2)?,
                    kind: WorkspaceKind::from_str(row.get_ref(3)?.as_str()?)
                        .map_err(to_sql_mapping_error)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|error| AppError::Runtime(format!("failed loading workspaces: {error}")))?;

        collect_rows(rows, "failed reading workspace row")
    }

    pub fn get_workspace(&self, workspace_id: &str) -> Result<Option<Workspace>, AppError> {
        let connection = self.open()?;
        self.init_schema(&connection)?;
        connection
            .query_row(
                "SELECT id, name, root_path, kind, created_at FROM workspaces WHERE id = ?1",
                params![workspace_id],
                |row| {
                    Ok(Workspace {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        root_path: row.get(2)?,
                        kind: WorkspaceKind::from_str(row.get_ref(3)?.as_str()?)
                            .map_err(to_sql_mapping_error)?,
                        created_at: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(|error| AppError::Runtime(format!("failed loading workspace: {error}")))
    }

    pub fn insert_managed_worktree(
        &self,
        input: NewManagedWorktree,
    ) -> Result<ManagedWorktree, AppError> {
        let worktree = ManagedWorktree {
            id: Uuid::new_v4().to_string(),
            workspace_id: input.workspace_id,
            branch_name: input.branch_name,
            source_ref: input.source_ref,
            path: input.path,
            created_at: now_rfc3339(),
        };

        let connection = self.open()?;
        self.init_schema(&connection)?;
        let result = connection.execute(
            "INSERT INTO managed_worktrees (id, workspace_id, branch_name, source_ref, path, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                worktree.id,
                worktree.workspace_id,
                worktree.branch_name,
                worktree.source_ref,
                worktree.path,
                worktree.created_at,
            ],
        );
        match result {
            Ok(_) => Ok(worktree),
            Err(error) if is_unique_violation(&error) => Err(AppError::conflict(
                "managed_branch_exists",
                format!(
                    "managed worktree branch already exists in workspace: {}",
                    worktree.branch_name
                ),
            )),
            Err(error) => Err(AppError::Runtime(format!(
                "failed inserting managed worktree: {error}"
            ))),
        }
    }

    pub fn list_managed_worktrees(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<ManagedWorktree>, AppError> {
        let connection = self.open()?;
        self.init_schema(&connection)?;
        let mut statement = connection
            .prepare(
                "SELECT id, workspace_id, branch_name, source_ref, path, created_at FROM managed_worktrees WHERE workspace_id = ?1 ORDER BY created_at DESC, branch_name ASC",
            )
            .map_err(|error| AppError::Runtime(format!("failed preparing worktree list query: {error}")))?;
        let rows = statement
            .query_map(params![workspace_id], |row| {
                Ok(ManagedWorktree {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    branch_name: row.get(2)?,
                    source_ref: row.get(3)?,
                    path: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|error| {
                AppError::Runtime(format!("failed loading managed worktrees: {error}"))
            })?;

        collect_rows(rows, "failed reading managed worktree row")
    }

    pub fn list_all_managed_worktrees(&self) -> Result<Vec<ManagedWorktree>, AppError> {
        let connection = self.open()?;
        self.init_schema(&connection)?;
        let mut statement = connection
            .prepare(
                "SELECT id, workspace_id, branch_name, source_ref, path, created_at FROM managed_worktrees ORDER BY created_at DESC, branch_name ASC",
            )
            .map_err(|error| AppError::Runtime(format!("failed preparing worktree list query: {error}")))?;
        let rows = statement
            .query_map([], |row| {
                Ok(ManagedWorktree {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    branch_name: row.get(2)?,
                    source_ref: row.get(3)?,
                    path: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|error| {
                AppError::Runtime(format!("failed loading managed worktrees: {error}"))
            })?;

        collect_rows(rows, "failed reading managed worktree row")
    }

    pub fn get_managed_worktree(
        &self,
        managed_worktree_id: &str,
    ) -> Result<Option<ManagedWorktree>, AppError> {
        let connection = self.open()?;
        self.init_schema(&connection)?;
        connection
            .query_row(
                "SELECT id, workspace_id, branch_name, source_ref, path, created_at FROM managed_worktrees WHERE id = ?1",
                params![managed_worktree_id],
                |row| {
                    Ok(ManagedWorktree {
                        id: row.get(0)?,
                        workspace_id: row.get(1)?,
                        branch_name: row.get(2)?,
                        source_ref: row.get(3)?,
                        path: row.get(4)?,
                        created_at: row.get(5)?,
                    })
                },
            )
            .optional()
            .map_err(|error| AppError::Runtime(format!("failed loading managed worktree: {error}")))
    }

    pub fn insert_session(&self, input: NewStoredSession) -> Result<StoredSession, AppError> {
        let session = StoredSession {
            id: input.id,
            name: input.name,
            runtime_name: input.runtime_name,
            kind: input.kind,
            workspace_id: input.workspace_id,
            managed_worktree_id: input.managed_worktree_id,
            created_at: now_rfc3339(),
        };

        let connection = self.open()?;
        self.init_schema(&connection)?;
        connection
            .execute(
                "INSERT INTO sessions (id, name, runtime_name, kind, workspace_id, managed_worktree_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    session.id,
                    session.name,
                    session.runtime_name,
                    session.kind.as_str(),
                    session.workspace_id,
                    session.managed_worktree_id,
                    session.created_at,
                ],
            )
            .map_err(|error| AppError::Runtime(format!("failed inserting session metadata: {error}")))?;
        Ok(session)
    }

    pub fn remove_session(&self, session_id: &str) -> Result<Option<StoredSession>, AppError> {
        let session = self.get_session(session_id)?;
        if session.is_none() {
            return Ok(None);
        }

        let connection = self.open()?;
        self.init_schema(&connection)?;
        connection
            .execute("DELETE FROM sessions WHERE id = ?1", params![session_id])
            .map_err(|error| {
                AppError::Runtime(format!("failed deleting session metadata: {error}"))
            })?;
        Ok(session)
    }

    pub fn get_session(&self, session_id: &str) -> Result<Option<StoredSession>, AppError> {
        let connection = self.open()?;
        self.init_schema(&connection)?;
        connection
            .query_row(
                "SELECT id, name, runtime_name, kind, workspace_id, managed_worktree_id, created_at FROM sessions WHERE id = ?1",
                params![session_id],
                |row| {
                    Ok(StoredSession {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        runtime_name: row.get(2)?,
                        kind: SessionKind::from_str(row.get_ref(3)?.as_str()?)
                            .map_err(to_sql_mapping_error)?,
                        workspace_id: row.get(4)?,
                        managed_worktree_id: row.get(5)?,
                        created_at: row.get(6)?,
                    })
                },
            )
            .optional()
            .map_err(|error| AppError::Runtime(format!("failed loading session metadata: {error}")))
    }

    pub fn list_sessions(&self) -> Result<Vec<StoredSession>, AppError> {
        let connection = self.open()?;
        self.init_schema(&connection)?;
        let mut statement = connection
            .prepare(
                "SELECT id, name, runtime_name, kind, workspace_id, managed_worktree_id, created_at FROM sessions ORDER BY created_at DESC, name ASC",
            )
            .map_err(|error| AppError::Runtime(format!("failed preparing session list query: {error}")))?;
        let rows = statement
            .query_map([], |row| {
                Ok(StoredSession {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    runtime_name: row.get(2)?,
                    kind: SessionKind::from_str(row.get_ref(3)?.as_str()?)
                        .map_err(to_sql_mapping_error)?,
                    workspace_id: row.get(4)?,
                    managed_worktree_id: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })
            .map_err(|error| AppError::Runtime(format!("failed loading sessions: {error}")))?;

        collect_rows(rows, "failed reading session row")
    }

    fn open(&self) -> Result<Connection, AppError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                AppError::Runtime(format!("failed creating store directory: {error}"))
            })?;
        }

        let connection = Connection::open(&self.path)
            .map_err(|error| AppError::Runtime(format!("failed opening sqlite store: {error}")))?;
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(|error| {
                AppError::Runtime(format!("failed enabling sqlite foreign keys: {error}"))
            })?;
        Ok(connection)
    }

    fn init_schema(&self, connection: &Connection) -> Result<(), AppError> {
        connection
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS workspaces (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    root_path TEXT NOT NULL UNIQUE,
                    kind TEXT NOT NULL CHECK (kind IN ('git', 'none')),
                    created_at TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS managed_worktrees (
                    id TEXT PRIMARY KEY,
                    workspace_id TEXT NOT NULL,
                    branch_name TEXT NOT NULL,
                    source_ref TEXT NOT NULL,
                    path TEXT NOT NULL UNIQUE,
                    created_at TEXT NOT NULL,
                    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
                    UNIQUE (workspace_id, branch_name)
                );

                CREATE TABLE IF NOT EXISTS sessions (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    runtime_name TEXT NOT NULL UNIQUE,
                    kind TEXT NOT NULL CHECK (kind IN ('local', 'worktree')),
                    workspace_id TEXT NOT NULL,
                    managed_worktree_id TEXT,
                    created_at TEXT NOT NULL,
                    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
                    FOREIGN KEY (managed_worktree_id) REFERENCES managed_worktrees(id) ON DELETE SET NULL
                );
                ",
            )
            .map_err(|error| AppError::Runtime(format!("failed initializing sqlite schema: {error}")))
    }
}

fn collect_rows<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> Result<T, SqlError>>,
    context: &str,
) -> Result<Vec<T>, AppError> {
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::Runtime(format!("{context}: {error}")))
}

fn is_unique_violation(error: &SqlError) -> bool {
    matches!(
        error,
        SqlError::SqliteFailure(code, _)
            if code.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE
                || code.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY
    )
}

fn to_sql_mapping_error(error: AppError) -> SqlError {
    SqlError::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

pub fn basename_for_path(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("workspace")
        .to_string()
}
