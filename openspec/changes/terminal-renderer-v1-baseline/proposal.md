## Why

AMUX has a daemon control-plane baseline, but no spec-level contract yet for browser terminal behavior. We need a clear v1 terminal capability contract now so renderer/input work can proceed with measurable mobile usability and performance targets.

## What Changes

- Add a new terminal web surface capability that defines renderer correctness, input behavior, and lifecycle expectations for canvas-based terminal sessions.
- Define a v1 mobile usability bar (iOS Safari and Android Chrome) for modifier input, orientation stability, and key delivery reliability.
- Define baseline terminal performance budgets and acceptance thresholds for latency, frame time, and sustained update handling.
- Define v1 browser support priorities and interoperability expectations that clients can rely on.

## Capabilities

### New Capabilities
- `terminal-web-surface-v1`: Browser terminal rendering and input contract for AMUX v1, including mobile usability and performance baselines.

### Modified Capabilities
- None.

## Impact

- Affects `amuxd` event contracts used by terminal clients and the Rust/WASM web terminal architecture.
- Affects terminal renderer implementation choices (`vte`, `vt100`, unicode helpers) and validation strategy.
- Establishes acceptance gates for cross-browser behavior, mobile modifier UX, and performance regression checks.
