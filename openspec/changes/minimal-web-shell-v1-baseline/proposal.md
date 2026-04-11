## Why

AMUX has a daemon control plane and a terminal renderer library, but no actual browser shell that turns those pieces into a usable product loop. Defining and building a minimal web shell now creates the first real end-to-end AMUX workflow and establishes a stable foundation for later dashboard, workspace, and streaming work.

## What Changes

- Add a new browser shell capability served by `amuxd` under `/app/...`.
- Introduce a separate shell app layer that consumes `amuxterm-web` rather than folding product-shell concerns into the renderer crate.
- Define a single-session shell workflow for listing, creating, selecting, interacting with, and terminating sessions.
- Define URL-addressable session routes for reload and direct navigation behavior.
- Define the baseline selected-session terminal transport as snapshot polling at a fixed visible-page cadence, with immediate refresh on selection and after input.
- Define graceful behavior when terminal routes are unavailable so the shell still functions as a session control surface.
- Require baseline mobile shell affordances, including touch-accessible session controls and mobile terminal modifier controls.

## Capabilities

### New Capabilities
- `web-session-shell-v1`: Browser shell for AMUX session lifecycle and selected-session terminal interaction, served by `amuxd` and backed by the existing daemon and terminal contracts.

### Modified Capabilities
- None.

## Impact

- Affects `amuxd` HTTP routing and static asset serving for the browser shell.
- Affects the new shell app layer that integrates session REST APIs, lifecycle WebSocket invalidation, and terminal polling/input behavior.
- Reuses `amuxterm-web` as a renderer/input library without changing the existing daemon JSON contracts.
- Establishes the first user-facing browser workflow that later terminal streaming, workspace/worktree features, and dashboard surfaces will build on.
