## 1. API Contract Foundation

- [ ] 1.1 Define shared API data models for session object, health response, and error envelope
- [ ] 1.2 Implement `GET /health` readiness endpoint
- [ ] 1.3 Implement `POST /sessions` create endpoint with stable session id response
- [ ] 1.4 Implement `GET /sessions` list endpoint with baseline session fields
- [ ] 1.5 Implement `GET /sessions/{session_id}` retrieval endpoint for active sessions
- [ ] 1.6 Implement `DELETE /sessions/{session_id}` termination endpoint with post-termination not-found semantics

## 2. Session Service Integration

- [ ] 2.1 Wire REST lifecycle handlers to session runtime service operations
- [ ] 2.2 Ensure terminated sessions are removed from active retrieval paths
- [ ] 2.3 Implement restart-time session rediscovery visibility for `GET /sessions`

## 3. Event Stream Baseline

- [ ] 3.1 Implement WebSocket endpoint at `GET /ws/events`
- [ ] 3.2 Define lifecycle event envelope with `event_id`, `event_type`, `occurred_at`, and `session_id`
- [ ] 3.3 Emit `session.created` events from successful create operations
- [ ] 3.4 Emit `session.terminated` events from successful termination operations

## 4. Error and Timestamp Consistency

- [ ] 4.1 Implement deterministic not-found error envelope for unknown or terminated session ids
- [ ] 4.2 Implement runtime-failure error envelope with stable machine-readable code
- [ ] 4.3 Standardize API and event timestamps to RFC3339 UTC strings

## 5. Local Validation

- [ ] 5.1 Validate health and lifecycle REST flows locally (`create -> list -> get -> terminate -> not-found`)
- [ ] 5.2 Validate WebSocket lifecycle delivery for `session.created` and `session.terminated`
- [ ] 5.3 Validate restart visibility by creating a running session, restarting daemon, and confirming `GET /sessions` continuity
