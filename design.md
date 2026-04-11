# AMUX Rewrite Design Decisions

## Purpose

Capture high-level architecture intent and two foundational decisions for the big-bang rewrite:

1. Terminal renderer strategy: Option C (Rust core + custom canvas renderer)
2. Runtime strategy: tmux-backed sessions first

This is an intent/decision document, not a formal specification.

## Product Intent

AMUX is rebuilt as a Rust full-stack command center for agentic coding:

- Web-first UI (no tty-rendered product interface)
- Persistent daemon runtime
- Remote access with strong keyboard support (including mobile modifier keys)
- First-class speech-to-text input for coding agents (device-agnostic clients)
- Workspace/worktree-first operation model
- Session-level attention visibility

## Architecture Direction

```text
┌───────────────────────────────────────────────────────┐
│                    amuxd (Rust)                       │
│ tokio + axum                                          │
├───────────────────────────────────────────────────────┤
│ workspace svc | worktree svc | session svc (tmux)     │
│ attention svc | auth svc | speech svc | event bus     │
├───────────────────────────────────────────────────────┤
│ REST API + WebSocket streams                          │
└───────────────────────┬───────────────────────────────┘
                        │
                        ▼
┌───────────────────────────────────────────────────────┐
│             Web Client (Leptos + WASM)                │
│ canvas dashboard + terminal surface + mobile modifiers│
└───────────────────────────────────────────────────────┘
```

---

## Decision 1: Terminal Renderer Strategy

### Decision

Adopt Option C for v1:

- Use Rust terminal ecosystem crates for parsing/state primitives
- Build a custom browser canvas renderer in Rust/WASM
- Build our own input model for keyboard/modifier behavior

### Context

The product requires custom UI composition around terminal sessions, including attention overlays, dashboard elements, and mobile affordances. We need stronger control than an off-the-shelf terminal widget exposes, but still want to stand on mature semantics where possible.

### Alternatives Considered

#### A) Pure custom renderer and terminal core (all from scratch)

- Pros: maximum control and purity
- Cons: highest complexity/risk; slowest time to first reliable interaction

#### B) Rust app with JS terminal core bridge

- Pros: fastest route to terminal correctness
- Cons: reduces Rust-only purity and increases cross-runtime complexity

#### C) Rust core + custom canvas renderer (chosen)

- Pros: balances control with reuse; keeps architecture Rust-first
- Cons: still substantial browser terminal engineering effort

### Consequences

- We own renderer behavior and UI composition from the start
- We still need to solve hard terminal UX problems (IME, selection, wide chars)
- Performance and correctness work become first-class engineering tracks

### Revisit Triggers

Revisit this decision if any of the following are true:

- Browser terminal correctness stalls progress for more than two milestones
- Input fidelity issues block practical daily usage
- Canvas rendering constraints force heavy JS interop anyway

If triggered, evaluate a scoped JS bridge for terminal rendering while preserving Rust control-plane and product architecture.

---

## Decision 2: Session Runtime Strategy

### Decision

Use tmux-backed sessions as the initial runtime substrate.

### Context

The system needs persistent, multiplexed, attach/detach-friendly sessions immediately. tmux already solves these fundamentals and reduces early risk while we build new daemon and web interaction layers.

### Alternatives Considered

#### A) Direct PTY session manager from day one

- Pros: maximum long-term flexibility
- Cons: delays product delivery with deep systems work early

#### B) tmux-backed sessions first (chosen)

- Pros: proven behavior; rapid path to reliable persistence
- Cons: tmux abstractions may constrain some UX later

### Consequences

- Faster path to usable product baseline
- We must define a runtime abstraction boundary so tmux can be replaced later if needed
- Some UX decisions may need to account for tmux semantics

### Revisit Triggers

Revisit this decision if any of the following are true:

- tmux limits features materially (session metadata, granular control, portability)
- Performance/reliability bottlenecks are traceable to tmux integration
- Product goals demand behavior tmux cannot provide cleanly

If triggered, plan migration to direct PTY runtime behind the existing session service interface.

---

## Non-Goals (Current Phase)

- No compatibility layer with legacy Go/TUI code
- No dual-runtime deployment path
- No immediate direct-PTY rewrite

## Milestone Skeleton (Planning Aid)

- M1: Rust daemon foundation (`tokio + axum`), session registry, event streaming
- M2: Leptos web shell and canvas dashboard
- M3: Terminal renderer v1 (output correctness, basic input)
- M4: Workspace/worktree lifecycle support
- M5: Mobile modifier UX and remote access hardening
- M6: Attention model and timeline/logbook polish

## Terminal Crate Shortlist (v0)

This shortlist is for Option C implementation planning (Rust core + custom canvas renderer).

### Parser / Escape Sequence Handling

#### `vte`

- Fit: mature ANSI/VT parser used in terminal ecosystem
- Pros: fast, battle-tested parsing primitive
- Cons: parser only; does not provide full terminal state model
- Recommendation: strong default for low-level parser layer

### Terminal State / Emulation

#### `vt100`

- Fit: stateful terminal emulation model suitable for rendering snapshots/diffs
- Pros: focused API, practical for building custom renderers
- Cons: feature surface may be narrower than full terminal emulators
- Recommendation: best first candidate for v1 state model

#### `alacritty_terminal`

- Fit: richer emulator internals from Alacritty codebase
- Pros: robust semantics and broad real-world coverage
- Cons: heavier integration, steeper complexity and coupling risk
- Recommendation: fallback if `vt100` proves insufficient

### PTY Abstraction (future direct runtime path)

#### `portable-pty`

- Fit: cross-platform PTY abstraction
- Pros: useful when/if migrating away from tmux-backed sessions
- Cons: not needed for tmux-first runtime in v1
- Recommendation: keep as migration-path dependency candidate, not immediate core

### Unicode / Width / Grapheme Support

#### `unicode-width` and `unicode-segmentation`

- Fit: character width and grapheme handling in renderer/input model
- Pros: standard Rust ecosystem choices for correctness basics
- Cons: terminal edge cases still require integration tests and patches
- Recommendation: include early; validate against tricky emoji/CJK fixtures

### Serialization / Event Payloads

#### `serde` + `serde_json`

- Fit: terminal event transport and snapshot payloads
- Pros: standard, ergonomic, low risk
- Cons: none material
- Recommendation: baseline choice

## Proposed v1 Terminal Core Stack

- Parser: `vte`
- State model: `vt100`
- Width/grapheme helpers: `unicode-width`, `unicode-segmentation`
- Transport: `serde`, `serde_json`

If this stack fails fidelity tests (IME, wrapping, edge ANSI behavior), next escalation is evaluating `alacritty_terminal` as the state core.

## Locked Decisions (Remaining)

### Decision 3: v1 Terminal Core Standard

Adopt the proposed stack as the v1 default:

- Parser: `vte`
- State model: `vt100`
- Width/grapheme helpers: `unicode-width`, `unicode-segmentation`
- Payload transport: `serde`, `serde_json`

Escalation path:

- Promote `alacritty_terminal` only if fidelity acceptance fails for two consecutive milestones on IME composition, wrap behavior, or ANSI edge-sequence fixtures.

### Decision 4: "Phone-Usable" Quality Bar for v1

Define phone-usable as all of the following on iOS Safari and Android Chrome:

- Text entry supports normal typing plus `Ctrl`, `Esc`, `Tab`, arrows, and `Enter` through modifier UX.
- No terminal corruption or cursor drift during copy/paste and orientation changes.
- Input reliability at or above 99.9% delivered key events over a 5,000-key scripted run.
- End-to-end keypress-to-visible-echo latency at p95 <= 160 ms on local/LAN and <= 280 ms on typical remote links.

### Decision 5: Cross-Browser Quirks Priority

Launch-priority browser matrix:

1. iOS Safari (highest risk and mobile-critical)
2. Android Chrome
3. Desktop Chromium
4. Desktop Firefox

Quirk triage order for v1:

- IME composition and virtual keyboard behavior
- Canvas text metrics and device-pixel-ratio scaling
- Selection/clipboard interoperability
- Focus restoration after reconnect and tab switching

### Decision 6: Performance Budget per Active Session

Set explicit v1 budgets per active terminal session:

- Server steady-state overhead: <= 8 MiB RSS per active session target.
- Server lifecycle overhead: <= 3% single-core CPU at idle for 20 active sessions aggregate.
- Client rendering: p95 frame time <= 16 ms during sustained output at 60 FPS target.
- Stream handling: sustain 2,000 cell updates/second per active session without dropped lifecycle events.

These budgets are enforced as milestone acceptance gates; any misses require either optimization work or explicit scope reductions before launch.
