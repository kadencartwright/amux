## ADDED Requirements

### Requirement: Browser shell session lifecycle workflow
The system SHALL provide a web shell that supports the baseline session lifecycle loop through the daemon control-plane contract.

#### Scenario: Load sessions in the shell
- **WHEN** the shell loads and the daemon is reachable
- **THEN** the shell retrieves and displays the current session list from `GET /sessions`

#### Scenario: Create session from the shell
- **WHEN** a user creates a session from the shell with valid input
- **THEN** the shell creates that session through the daemon control-plane contract
- **AND** the created session appears in the shell session list

#### Scenario: Terminate session from the shell
- **WHEN** a user terminates an existing session from the shell
- **THEN** the shell terminates that session through the daemon control-plane contract
- **AND** the terminated session is removed from active shell retrieval

### Requirement: Daemon-served shell routing
The system SHALL expose the minimal web shell through daemon-served browser routes under `/app/...`.

#### Scenario: Shell entry route
- **WHEN** a browser requests `GET /app`
- **THEN** the daemon returns the shell entrypoint for the browser application

#### Scenario: Selected-session shell route
- **WHEN** a browser requests `GET /app/sessions/{session_id}`
- **THEN** the daemon returns the shell entrypoint for the browser application

#### Scenario: Shell asset route
- **WHEN** a browser requests `GET /app/assets/*`
- **THEN** the daemon serves the requested shell static asset when it exists

### Requirement: Single selected session terminal view
The system SHALL let the user select one active session at a time and render that session's terminal surface in the main shell pane.

#### Scenario: Select session from session list
- **WHEN** a user selects an existing session in the shell
- **THEN** the shell marks that session as the active session
- **AND** the main pane renders that session's terminal surface

#### Scenario: No session selected
- **WHEN** the shell has no active session selected
- **THEN** the main pane presents a non-terminal empty state instead of attempting terminal polling or input submission

### Requirement: Terminal rendering contract reuse
The system SHALL render the selected session using the existing terminal surface contract without exposing tmux-specific details to the browser shell.

#### Scenario: Render selected terminal surface
- **WHEN** the shell loads terminal state for the selected session
- **THEN** it consumes the daemon terminal surface contract for that session
- **AND** it does not require tmux pane ids, tmux command names, or other tmux-specific client fields

### Requirement: Route-addressable session selection
The system SHALL use the browser route as the canonical representation of the selected shell session.

#### Scenario: Create auto-selects new session
- **WHEN** a user creates a session successfully from the shell
- **THEN** the shell navigates to `/app/sessions/{new_session_id}`
- **AND** the created session becomes the selected session

#### Scenario: Reload restores selected session
- **WHEN** a user reloads `/app/sessions/{session_id}` for an existing session
- **THEN** the shell restores that session as the selected session

#### Scenario: Selected session no longer exists
- **WHEN** the shell loads or refreshes `/app/sessions/{session_id}` and that session no longer exists
- **THEN** the shell normalizes to `/app`
- **AND** it shows a non-blocking session-unavailable state

### Requirement: Session lifecycle awareness via event invalidation
The system SHALL keep shell session state fresh by combining REST reads with lifecycle-event-driven invalidation.

#### Scenario: Refresh session list after lifecycle event
- **WHEN** the shell receives a lifecycle event from `GET /ws/events`
- **THEN** the shell refetches the session list from the daemon

#### Scenario: Recover session list after reconnect
- **WHEN** the lifecycle event connection is interrupted and later restored
- **THEN** the shell refetches the session list from the daemon before resuming normal observation

### Requirement: Selected-session terminal polling baseline
The system SHALL use snapshot polling as the baseline terminal transport for the selected session until terminal streaming exists.

#### Scenario: Poll selected visible session
- **WHEN** a session is selected and the shell page is visible
- **THEN** the shell polls `GET /sessions/{session_id}/terminal` for that selected session on a repeating `250 ms` baseline cadence

#### Scenario: Immediate refresh on session selection
- **WHEN** a user selects a different session
- **THEN** the shell immediately requests that session's terminal surface before the next polling interval

#### Scenario: Resume polling after visibility restoration
- **WHEN** the shell page becomes visible again while a session is selected
- **THEN** the shell immediately requests that session's terminal surface
- **AND** resumes the repeating polling cadence

#### Scenario: Pause polling when inactive
- **WHEN** no session is selected or the shell page is hidden
- **THEN** the shell stops terminal polling until the session becomes active and visible again

### Requirement: Terminal input submission from the shell
The system SHALL allow the selected session to receive terminal input through the existing terminal input contract.

#### Scenario: Send text and key input
- **WHEN** a user enters supported terminal input for the selected session
- **THEN** the shell submits that input through `POST /sessions/{session_id}/terminal/input`

#### Scenario: Refresh terminal after successful input
- **WHEN** terminal input submission succeeds for the selected session
- **THEN** the shell triggers an immediate terminal refresh for that session

### Requirement: Graceful terminal-unavailable behavior
The system SHALL remain usable as a session shell even when daemon terminal routes are disabled or unavailable.

#### Scenario: Terminal feature unavailable for selected session
- **WHEN** the shell selects a session but the daemon does not expose the terminal surface route for that environment
- **THEN** the shell shows a terminal-unavailable state for that session
- **AND** session list, create, and terminate actions remain usable

### Requirement: Mobile shell affordances
The system SHALL provide mobile-usable shell controls and touch-accessible terminal modifiers in this baseline.

#### Scenario: Session controls remain reachable on phone-sized widths
- **WHEN** the shell is rendered on a phone-sized viewport
- **THEN** the terminal remains the primary pane
- **AND** session create, select, and terminate controls remain reachable through a mobile-appropriate layout

#### Scenario: Session list collapses on phone-sized widths
- **WHEN** the shell is rendered on a phone-sized viewport
- **THEN** the session list is presented through a drawer, sheet, or similarly collapsible control surface rather than a permanently open rail

#### Scenario: Mobile modifier controls are available
- **WHEN** a user interacts with the selected session on a mobile device
- **THEN** dedicated touch-accessible controls are available for `Ctrl`, `Esc`, `Tab`, arrows, and `Enter`

### Requirement: Minimal shell scope boundary
The system SHALL keep this shell baseline intentionally narrow and exclude higher-level product surfaces from the required behavior.

#### Scenario: Out-of-scope product areas
- **WHEN** this baseline shell is evaluated for completion
- **THEN** workspace/worktree controls, attention dashboards, timeline/logbook views, auth flows, terminal streaming, and multi-pane shell layouts are not required for this capability
