## Context

`amuxd` needs a minimal but stable control plane that can be exercised locally before UI and terminal rendering work is complete. The runtime layer is tmux-backed in v1, but clients should interact through a backend-owned contract rather than tmux-specific semantics. This change defines the baseline REST and WebSocket surfaces and the shared payload conventions needed for early manual testing and client integration.

## Goals / Non-Goals

**Goals:**
- Define a small REST surface for session lifecycle control and querying.
- Define a WebSocket lifecycle event stream with a required envelope and core event types.
- Standardize error envelope behavior for common failure paths.
- Keep behavior locally testable with common tools (`curl` + WebSocket client).

**Non-Goals:**
- Authentication/authorization hardening.
- Terminal rendering fidelity, input model details, or stream rendering protocol.
- Exactly-once event delivery or distributed event guarantees.
- Session tombstone retention policies beyond baseline behavior.

## Decisions

1. **REST-first control plane baseline**
   - Decision: expose `GET /health`, `POST /sessions`, `GET /sessions`, `GET /sessions/{session_id}`, and `DELETE /sessions/{session_id}`.
   - Rationale: these operations provide the minimum viable lifecycle loop for local validation and early client development.
   - Alternatives considered:
     - Fewer endpoints (`/sessions` only): rejected because direct by-id read and explicit health checks are needed for deterministic testing.
     - Broader API now (attach, logs, replay): rejected to keep first implementation narrow and shippable.

2. **Lifecycle observation over a single WebSocket endpoint**
   - Decision: expose `GET /ws/events` for lifecycle events with a required event envelope.
   - Rationale: a single subscription point is easy to integrate and debug locally while preserving extensibility for future event types.
   - Alternatives considered:
     - Polling-only model: rejected due to delayed visibility and poor ergonomics during rapid local testing.
     - Separate streams per event class: rejected as unnecessary complexity for baseline.

3. **Post-termination visibility behavior**
   - Decision: terminated sessions are removed from active retrieval in this baseline; `GET /sessions/{session_id}` returns not-found after termination.
   - Rationale: avoids early retention/tombstone complexity and keeps behavior straightforward.
   - Alternatives considered:
     - Tombstone retention window: rejected for baseline due to extra state management and policy decisions not needed for first working slice.

4. **Timestamp format and event baseline**
   - Decision: use RFC3339 UTC timestamps across API and event payloads; require `session.created` and `session.terminated` for baseline, with `session.rediscovered` deferred.
   - Rationale: RFC3339 is human-readable and easy to debug; deferring rediscovery events reduces scope while preserving restart validation via REST.
   - Alternatives considered:
     - Epoch milliseconds: rejected for baseline due to lower readability in manual debugging.
     - Require rediscovered event now: rejected to reduce first-slice coupling between restart and stream behavior.

## Risks / Trade-offs

- [Event delivery guarantees are minimal in baseline] -> Document at-least-once expectations in spec and avoid exactly-once assumptions in initial clients.
- [No post-termination tombstones may reduce observability] -> Keep consistent not-found behavior and revisit retention in a follow-up capability.
- [Deferred rediscovery events may limit real-time restart insight] -> Require post-restart visibility through `GET /sessions` and add rediscovery events in a later change.
- [Contract drift between runtime and API layers] -> Enforce tmux-independent API semantics in spec language and acceptance scenarios.

## Migration Plan

1. Implement baseline REST routes and shared response/error envelope types.
2. Implement WebSocket lifecycle publisher for baseline event types.
3. Wire runtime lifecycle transitions to API state and event emission.
4. Validate local manual test scenarios (health, lifecycle loop, event observation, restart visibility).
5. Expand contract in follow-up changes only after baseline behavior is stable.

Rollback strategy:
- If event stream introduces instability, keep REST endpoints enabled and temporarily disable WebSocket publication while preserving API behavior.

## Locked Decisions

1. **`GET /sessions` pagination in baseline**
   - Decision: keep baseline unpaginated for now, but define deterministic ordering by `created_at` descending.
   - Forward-compatibility rule: future pagination is additive via explicit query params and does not change default ordering semantics.
   - Rationale: baseline scope stays simple while still giving clients stable, testable list behavior.

2. **Lifecycle event ordering semantics in baseline**
   - Decision: formalize minimal guarantees now.
   - Guarantees:
     - Per session id: lifecycle events are emitted in causal order (`session.created` before `session.terminated`).
     - Across different sessions: no global total ordering guarantee.
     - Delivery model: at-least-once; clients must tolerate duplicates via `event_id` de-duplication.
   - Rationale: this removes ambiguity for client behavior without requiring heavyweight distributed ordering guarantees.
