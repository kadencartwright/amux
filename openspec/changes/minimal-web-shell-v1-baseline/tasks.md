## 1. Shell Foundation

- [ ] 1.1 Add a daemon-served shell entrypoint and static asset routing under `/app`, without changing existing REST or WebSocket API behavior.
- [ ] 1.2 Create a separate shell app layer that consumes `amuxterm-web` and can be built/served as the `/app` browser client.
- [ ] 1.3 Define shell route/state types for `/app` and `/app/sessions/{session_id}`, including selected-session normalization when a route session no longer exists.

## 2. Session Shell Workflow

- [ ] 2.1 Implement initial shell boot flow to fetch `GET /sessions`, subscribe to `GET /ws/events`, and resolve the selected session from the current route.
- [ ] 2.2 Implement session list rendering plus create/select/terminate controls backed by the existing daemon lifecycle APIs.
- [ ] 2.3 Implement create auto-selection so successful session creation refetches the list, navigates to `/app/sessions/{new_session_id}`, and focuses terminal input.
- [ ] 2.4 Implement REST-authoritative session refresh behavior after lifecycle events and after WebSocket reconnect.

## 3. Selected Session Terminal

- [ ] 3.1 Implement selected-session terminal rendering in the shell using the existing terminal surface contract and `amuxterm-web`.
- [ ] 3.2 Implement selected-session polling at `250 ms` while the page is visible, including immediate refresh on selection change and on visibility restoration.
- [ ] 3.3 Implement terminal input submission through the existing terminal input API with immediate terminal refresh after successful input.
- [ ] 3.4 Implement graceful terminal-unavailable and empty-state behavior so session lifecycle controls remain usable when no terminal surface is available.

## 4. Mobile And Routing UX

- [ ] 4.1 Implement route-addressable selected-session behavior for direct navigation, reload restore, and normalization back to `/app` when the selected session is gone.
- [ ] 4.2 Implement a mobile layout where the session list collapses into a drawer or sheet and session controls remain touch-accessible.
- [ ] 4.3 Implement dedicated mobile terminal modifier controls for `Ctrl`, `Esc`, `Tab`, arrows, and `Enter` using the existing terminal input contract.

## 5. Verification

- [ ] 5.1 Add backend route tests covering `/app`, `/app/sessions/{session_id}`, and shell asset serving without API route regressions.
- [ ] 5.2 Add shell tests for session lifecycle flow, route restoration, polling start/stop behavior, and REST refetch on lifecycle events.
- [ ] 5.3 Add mobile-focused shell tests for collapsed session navigation and modifier control availability.
- [ ] 5.4 Add a manual verification path that exercises desktop and mobile-sized browser workflows: create, select, input, observe terminal updates, terminate, reload, and terminal-unavailable fallback.
