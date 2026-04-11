## Why

AMUX needs a stable local control-plane contract before frontend terminal work can proceed safely. Defining a minimal REST and WebSocket baseline now enables rapid manual validation, keeps client work unblocked, and reduces ambiguity in session lifecycle behavior.

## What Changes

- Add a new daemon API baseline for session lifecycle operations (`health`, create, list, get, terminate).
- Add a new lifecycle event stream baseline over WebSocket with a required event envelope and core session event types.
- Define deterministic error envelope behavior for not-found and runtime failures.
- Define local restart visibility expectations for running sessions through the API.

## Capabilities

### New Capabilities
- `daemon-api-event-stream-baseline`: Local-first REST and WebSocket contracts for session lifecycle control and observation.

### Modified Capabilities
None.

## Impact

- Affects `amuxd` HTTP API surface and WebSocket event stream surface.
- Affects session service integration points and lifecycle event production.
- Establishes payload and error shape conventions for early CLI/web clients.
- Creates a testable local baseline for future capabilities (workspace/worktree, terminal surface, attention model).
