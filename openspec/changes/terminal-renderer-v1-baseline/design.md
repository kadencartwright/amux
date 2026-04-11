## Context

AMUX currently has a daemon API/event baseline and a tmux-backed runtime contract, but terminal interaction in the web client is not yet specified as a capability. The product direction is Rust-first with a custom browser canvas terminal surface, strong mobile keyboard support, and session-centric remote interaction. This change defines the technical baseline needed to implement that surface without coupling clients to tmux internals.

## Goals / Non-Goals

**Goals:**
- Define the v1 renderer/input architecture boundary between daemon events and the WASM canvas terminal.
- Standardize the v1 core stack (`vte`, `vt100`, `unicode-width`, `unicode-segmentation`) and escalation criteria.
- Define measurable mobile usability and performance acceptance gates for v1.
- Define browser support and quirk triage priorities to guide implementation order.

**Non-Goals:**
- Full terminal feature parity with mature desktop emulators on day one.
- Speech-to-text protocol details, auth hardening, or workspace/worktree workflows.
- Direct-PTY runtime migration (tmux-backed sessions remain baseline runtime).
- Exactly-once stream delivery guarantees.

## Decisions

1. **Terminal state/rendering pipeline**
   - Decision: use `vte` for escape parsing and `vt100` for terminal state snapshots/diffs; render via Rust/WASM canvas layer.
   - Rationale: keeps a Rust-first architecture while avoiding fully custom emulation complexity.
   - Alternatives considered:
     - All-custom parser/emulator: rejected for timeline and correctness risk.
     - JS terminal core bridge: rejected for cross-runtime complexity and reduced Rust ownership.

2. **Input model and mobile modifier surface**
   - Decision: define a backend-agnostic input contract that supports text entry plus `Ctrl`, `Esc`, `Tab`, arrows, and `Enter` from touch-first modifier UI.
   - Rationale: mobile usability is a product requirement and must be spec-visible, not implicit UI behavior.
   - Alternatives considered:
     - Desktop-first key model with partial mobile fallback: rejected due to weak phone viability.

3. **Phone-usable acceptance bar**
   - Decision: v1 acceptance requires successful runs on iOS Safari and Android Chrome with no corruption on orientation/copy-paste transitions and >= 99.9% key delivery over scripted runs.
   - Rationale: a concrete quality bar prevents subjective "looks usable" decisions.
   - Alternatives considered:
     - Qualitative-only UX acceptance: rejected because it cannot gate regressions.

4. **Performance budgets as release gates**
   - Decision: enforce explicit latency/frame/update budgets for active sessions (keypress echo p95, frame time p95, sustained update throughput).
   - Rationale: performance/correctness are first-class tracks in this architecture and must be contractually testable.
   - Alternatives considered:
     - Post-hoc optimization after feature completion: rejected due to high rework risk.

5. **Browser priority and quirk triage**
   - Decision: prioritize iOS Safari first, then Android Chrome, then desktop Chromium and Firefox; triage IME/virtual keyboard, canvas metrics, clipboard/selection, then focus restoration.
   - Rationale: highest-risk environments should shape early implementation choices.
   - Alternatives considered:
     - Desktop-first compatibility pass: rejected as misaligned with mobile-critical product goals.

## Risks / Trade-offs

- [IME and virtual keyboard edge behavior differs across browsers] -> Build fixture-driven acceptance tests and prioritize iOS Safari parity early.
- [Canvas text metrics can drift with DPR/font differences] -> Use deterministic cell measurement at session start and recalibrate on viewport/orientation changes.
- [At-least-once stream delivery introduces duplicate updates] -> Require event idempotency/de-dup handling in client update pipeline.
- [Strict performance gates may reduce initial feature scope] -> Treat budgets as release constraints and defer non-critical UX polish when needed.

## Migration Plan

1. Define capability-level spec requirements for renderer correctness, input, browser support, and budgets.
2. Implement terminal adapter boundary from daemon events to renderer state updates.
3. Ship baseline mobile modifier input path and key mapping validation.
4. Add benchmark and fixture suites for latency, throughput, and cross-browser behaviors.
5. Roll out behind a feature flag until acceptance scenarios pass on required browsers.

Rollback strategy:
- If v1 renderer path is unstable, disable web terminal surface flag while retaining daemon control-plane capabilities.

## Open Questions

- What initial fixture corpus should define "ANSI edge-sequence" coverage for v1?
- Do we need a separate degraded-mode budget for low-power mobile devices, or only one global budget set?
