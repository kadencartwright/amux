## Context

AMUX has a usable daemon baseline and a terminal renderer library, but it does not yet have a real web product shell that ties those pieces together into an end-to-end workflow. The next logical slice is not a full dashboard; it is a thin browser shell that proves the system can be used through `amuxd` to create, select, interact with, and terminate sessions.

This change defines that shell as a constrained baseline so we can get to a usable product slice quickly without prematurely locking in terminal streaming, workspace/worktree flows, or broader dashboard structure. The shell is served from the daemon under `/app/...`, uses a separate app layer on top of `amuxterm-web`, and includes baseline mobile affordances.

## Goals / Non-Goals

**Goals:**
- Define the smallest real AMUX web shell that exercises the existing daemon and terminal contracts end to end.
- Keep the app architecture aligned with the repo-level design direction: web-first, Rust-first, and renderer-library separation.
- Define a stable shell URL model and daemon-served browser entrypoint.
- Specify how session list state, terminal state, and terminal input work before terminal streaming exists.
- Make the shell usable enough for daily local validation and early dogfooding on desktop and mobile.

**Non-Goals:**
- Full dashboard composition, attention surfaces, or timeline/logbook views.
- Workspace/worktree lifecycle UI.
- Auth, remote hardening, or multi-user concerns.
- A streaming terminal transport; snapshot polling is acceptable in this baseline.
- Multi-pane session layout.

## Decisions

1. **Separate app shell from renderer library**
   - Decision: keep `amuxterm-web` as the terminal renderer/input library and introduce the minimal web shell as a separate app layer that consumes its contract.
   - Rationale: this preserves a clean boundary between reusable terminal primitives and product-specific UI state, routing, and API integration.
   - Alternatives considered:
     - Fold app-shell concerns into `amuxterm-web`: rejected because it would blur renderer and application responsibilities too early.

2. **Serve the shell from `amuxd` under `/app/...`**
   - Decision: the browser shell is served by the daemon under a dedicated namespace with:
     - `/app` as the shell entry route
     - `/app/sessions/{session_id}` as the canonical selected-session route
     - `/app/assets/*` for shell assets
   - Rationale: one process and one URL gives the first real product slice the simplest local deployment model while avoiding conflicts with the existing REST API paths.
   - Alternatives considered:
     - Separate client app/dev server: rejected because it weakens the integrated product baseline.
     - Root-mounted shell at `/`: rejected because `/app/...` makes route separation with the existing API simpler and more explicit.

3. **Single-session-focused shell for v1 baseline**
   - Decision: the shell centers on one active session at a time, with a session list/control rail and one terminal surface in the main pane.
   - Rationale: this is enough to prove browser usability without committing to the larger command-center layout prematurely.
   - Alternatives considered:
     - Full multi-panel dashboard now: rejected because it expands scope before the first end-to-end shell exists.

4. **Server-authoritative session state with refetch on lifecycle events**
   - Decision: bootstrap session state from `GET /sessions`, use `GET /ws/events` only as an invalidation signal, and refetch the session list after lifecycle events rather than maintaining a richer client-side event projection.
   - Rationale: the daemon already exposes a stable session list contract, and refetch-on-event keeps the client simple and deterministic.
   - Alternatives considered:
     - Fully event-driven client session store: rejected because it creates more client state complexity than this baseline needs.

5. **Snapshot polling for the selected terminal session**
   - Decision: until terminal streaming exists, the shell polls `GET /sessions/{session_id}/terminal` for the currently selected session only.
   - Baseline behavior:
     - Fetch immediately when a session is selected.
     - Poll every 250 ms while that session is selected and the page is visible.
     - Refetch immediately when page visibility returns to visible.
     - Stop polling when no session is selected or the page is hidden.
     - Trigger an immediate refresh after successful terminal input submission.
   - Rationale: this gives a usable browser loop now while keeping the transport replaceable later.
   - Alternatives considered:
     - Wait for terminal streaming first: rejected because it delays the first usable shell.
     - Poll all sessions: rejected due to unnecessary server/client load and poor scaling.

6. **Route-level session selection**
   - Decision: the selected session is represented in the route rather than in client memory only.
   - Required behaviors:
     - creating a session auto-selects it and navigates to `/app/sessions/{new_session_id}`
     - reloading or directly visiting `/app/sessions/{session_id}` restores selection when the session exists
     - if the selected session no longer exists, the shell normalizes to `/app` and shows a non-blocking unavailable message
   - Rationale: route-level selection makes shell state durable and debuggable without needing additional persistence mechanisms.
   - Alternatives considered:
     - Ephemeral in-memory selection: rejected because it weakens reload/deep-link behavior for little gain.

7. **Graceful degradation when terminal routes are disabled**
   - Decision: if terminal renderer routes are unavailable on the daemon, the shell still renders session lifecycle controls and shows a non-broken terminal-unavailable state for the selected session.
   - Rationale: the control-plane shell should remain debuggable even when the terminal feature flag is off.
   - Alternatives considered:
     - Hard-fail the shell when terminal routes are absent: rejected because it makes baseline troubleshooting worse.

8. **Mobile shell affordances are in scope**
   - Decision: this baseline includes more than responsive layout; it requires mobile-usable shell controls and dedicated terminal modifier controls for `Ctrl`, `Esc`, `Tab`, arrows, and `Enter`.
   - Required behaviors:
     - on phone-sized widths, the terminal remains the primary pane
     - the session list collapses into a drawer or sheet
     - create/select/terminate controls remain touch-accessible
     - dedicated terminal modifier controls are reachable on mobile
   - Rationale: the product direction is mobile-capable remote access, and the first real shell should not defer all touch interaction to a later phase.
   - Alternatives considered:
     - Responsive layout only: rejected because it would not prove meaningful mobile usage.
     - Defer mobile completely: rejected because it would undercut a key product promise.

## Risks / Trade-offs

- [Polling is simpler but less efficient than streaming] -> Keep the polling scope to the selected visible session only and treat streaming as the next transport upgrade.
- [Single-session focus may bias later dashboard layout] -> Treat this shell as a baseline workflow slice, not a final information architecture.
- [Refetch-on-event can momentarily lag local optimistic state] -> Prefer server-authoritative consistency over richer client-side projection in this baseline.
- [Mobile inclusion expands baseline scope] -> Keep the mobile requirement focused on shell usability and modifier availability rather than full fidelity hardening.
- [Daemon-served shell introduces asset/routing concerns] -> Keep the `/app/...` namespace explicit and isolated from existing API routes.

## Migration Plan

1. Add a new web-shell capability spec for the baseline browser workflow.
2. Implement a thin web app layer that consumes existing daemon REST/WebSocket routes and the terminal renderer library.
3. Extend `amuxd` to serve shell routes and browser assets under `/app/...`.
4. Ship session lifecycle actions and selected-session terminal view behind the existing terminal feature flag behavior.
5. Validate the browser loop locally on desktop and mobile-sized layouts: create, select, interact, observe output, terminate, refresh/reload.
6. Follow with terminal streaming and broader dashboard/workspace capabilities in later changes.

Rollback strategy:
- If the shell is unstable, keep the daemon and terminal library intact and disable only the app-shell entrypoint while preserving backend contracts.

## Locked Decisions

1. **Transport boundary for this chunk**
   - Decision: do not introduce a new terminal streaming transport in this change.
   - Forward-compatibility rule: the shell must consume the existing terminal surface contract in a way that can later swap polling for streaming without changing renderer semantics.
   - Rationale: this preserves momentum and keeps the chunk bounded.

2. **Session selection model**
   - Decision: only one session is actively rendered at a time in this baseline, and background sessions are represented in the list only.
   - Rationale: it keeps terminal rendering, polling, and input routing straightforward.

3. **Routing namespace**
   - Decision: the shell lives under `/app/...` rather than `/`.
   - Rationale: it avoids ambiguity with the daemon API and keeps browser routing explicit.

4. **Selection persistence**
   - Decision: selection persistence is route-based, not local-storage-based or memory-only.
   - Rationale: the route is the canonical selected-session source for reloads and direct links.

5. **Shell scope boundary**
   - Decision: workspace/worktree controls, attention signals, timeline/logbook views, auth flows, and multi-pane layouts are explicitly excluded from this baseline shell.
   - Rationale: those features should layer on top of a proven browser control loop, not be designed into the first usable shell slice.
