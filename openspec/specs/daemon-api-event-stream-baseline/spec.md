# daemon-api-event-stream-baseline Specification

## Purpose
TBD - created by archiving change daemon-api-event-stream-baseline. Update Purpose after archive.
## Requirements
### Requirement: Daemon health endpoint
The system SHALL expose a health endpoint that reports daemon readiness for local control-plane use.

#### Scenario: Health endpoint returns ready when operational
- **WHEN** a client sends `GET /health` while core daemon services are operational
- **THEN** the system returns a successful readiness response

### Requirement: Session lifecycle REST API
The system SHALL provide REST endpoints to create, list, retrieve, and terminate sessions using stable session identifiers.

#### Scenario: Create session
- **WHEN** a client sends valid input to `POST /sessions`
- **THEN** the system creates a session and returns a session object with a stable `id`

#### Scenario: List sessions
- **WHEN** a client sends `GET /sessions`
- **THEN** the system returns known sessions including baseline fields (`id`, `name`, `state`, `created_at`, `last_activity_at`)
- **AND** the returned list is ordered by `created_at` descending

#### Scenario: Retrieve session by id
- **WHEN** a client sends `GET /sessions/{session_id}` for an existing active session
- **THEN** the system returns that session object

#### Scenario: Terminate session
- **WHEN** a client sends `DELETE /sessions/{session_id}` for an existing active session
- **THEN** the system terminates that session and removes it from active retrieval

### Requirement: Post-termination retrieval semantics
The system SHALL return deterministic not-found behavior when a client references a non-existent session, including sessions that were terminated in this baseline.

#### Scenario: Retrieve terminated session id
- **WHEN** a client sends `GET /sessions/{session_id}` after that session has been terminated
- **THEN** the system returns a not-found error envelope

#### Scenario: Terminate unknown session id
- **WHEN** a client sends `DELETE /sessions/{session_id}` for an unknown id
- **THEN** the system returns a not-found error envelope

### Requirement: Error envelope consistency
The system SHALL return machine-parseable error responses with a stable envelope containing `error.code` and `error.message` for baseline failure cases.

#### Scenario: Runtime operation failure
- **WHEN** a lifecycle operation fails due to runtime backend failure
- **THEN** the system returns an error envelope with a stable runtime-failure `error.code`

### Requirement: WebSocket lifecycle event stream
The system SHALL expose a WebSocket endpoint for session lifecycle observation and emit baseline lifecycle events using a consistent event envelope.

#### Scenario: Subscribe to lifecycle stream
- **WHEN** a client connects to `GET /ws/events` and the protocol upgrades to WebSocket
- **THEN** the system establishes a lifecycle event stream

#### Scenario: Emit created event
- **WHEN** a session is created through the control plane
- **THEN** the system emits a `session.created` event envelope containing `event_id`, `event_type`, `occurred_at`, and `session_id`

#### Scenario: Emit terminated event
- **WHEN** a session is terminated through the control plane
- **THEN** the system emits a `session.terminated` event envelope containing `event_id`, `event_type`, `occurred_at`, and `session_id`

### Requirement: Lifecycle event ordering and delivery semantics
The system SHALL provide minimal, explicit lifecycle event semantics suitable for baseline client behavior.

#### Scenario: Per-session causal ordering
- **WHEN** lifecycle events are emitted for the same `session_id`
- **THEN** `session.created` is emitted before `session.terminated` for that session

#### Scenario: Cross-session ordering
- **WHEN** lifecycle events are emitted for different session ids
- **THEN** the system does not guarantee a global total ordering across those sessions

#### Scenario: At-least-once delivery behavior
- **WHEN** a client consumes lifecycle events from the WebSocket stream
- **THEN** the client may receive duplicates and can de-duplicate by `event_id`

### Requirement: Timestamp format baseline
The system SHALL serialize baseline API and event timestamps as RFC3339 UTC strings.

#### Scenario: API timestamp format
- **WHEN** a client reads session timestamp fields from REST responses
- **THEN** each timestamp is encoded as an RFC3339 UTC string

#### Scenario: Event timestamp format
- **WHEN** a client receives lifecycle events from the WebSocket stream
- **THEN** `occurred_at` is encoded as an RFC3339 UTC string

### Requirement: Restart visibility through REST
The system SHALL make running sessions visible through `GET /sessions` after daemon restart once readiness is restored.

#### Scenario: Session remains visible after restart
- **WHEN** a running session exists, the daemon restarts, and a client sends `GET /sessions` after readiness
- **THEN** the previously running session appears in the returned session list

### Requirement: Runtime implementation independence
The system SHALL keep API and event contracts independent of tmux-specific details.

#### Scenario: Client consumes contract without tmux knowledge
- **WHEN** a client performs baseline lifecycle operations and consumes lifecycle events
- **THEN** the client can do so without relying on tmux command names, pane ids, or tmux-specific payload fields

