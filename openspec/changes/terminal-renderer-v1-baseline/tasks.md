## 1. Terminal Core and Contracts

- [ ] 1.1 Add terminal core module wiring `vte`, `vt100`, `unicode-width`, and `unicode-segmentation` behind a renderer-facing interface
- [ ] 1.2 Define backend-agnostic terminal event/input contract types with no tmux-specific fields
- [ ] 1.3 Add fallback evaluation hook and acceptance criteria for escalating state core to `alacritty_terminal`

## 2. Canvas Renderer and Input Path

- [ ] 2.1 Implement WASM canvas render loop consuming terminal state snapshots/diffs
- [ ] 2.2 Implement baseline mobile modifier UX mappings for `Ctrl`, `Esc`, `Tab`, arrows, and `Enter`
- [ ] 2.3 Implement orientation-change and copy/paste stability handling to prevent cursor drift/corruption

## 3. Cross-Browser and Reliability Validation

- [ ] 3.1 Add browser matrix test plan and fixtures for iOS Safari, Android Chrome, desktop Chromium, and desktop Firefox
- [ ] 3.2 Add scripted 5,000-key reliability test harness with pass/fail threshold at 99.9% delivery
- [ ] 3.3 Add IME, virtual keyboard, clipboard/selection, and focus-restoration interoperability checks

## 4. Performance Gates and Rollout

- [ ] 4.1 Add latency measurement harness and enforce p95 keypress-to-echo budgets for local/LAN and remote links
- [ ] 4.2 Add renderer frame-time and throughput benchmarks for 16 ms p95 frame target and 2,000 updates/sec load
- [ ] 4.3 Gate release behind feature flag and verify all spec acceptance scenarios before enabling by default
