# Terminal Browser Matrix

This document is the baseline validation plan for `terminal-renderer-v1-baseline`.

## Priority Order

1. iOS Safari
2. Android Chrome
3. Desktop Chromium
4. Desktop Firefox

## Fixture Corpus

Use [amuxterm-web/fixtures/browser-fixtures.json](/home/k/code/amux/amuxterm-web/fixtures/browser-fixtures.json) as the minimum ANSI fixture set for every browser run.

## Required Checks

For each browser in priority order:

1. Launch `amuxd` with `AMUXD_TERMINAL_RENDERER_V1=1`.
2. Render every fixture from `browser-fixtures.json` through the terminal surface.
3. Verify the mobile modifier path for `Ctrl`, `Esc`, `Tab`, arrows, and `Enter`.
4. Verify orientation change or viewport resize forces a clean repaint with no cursor drift.
5. Verify copy/paste preserves text integrity and does not duplicate or corrupt pasted content.
6. Verify IME composition can start, commit, and recover after focus loss.
7. Verify focus restoration after backgrounding the tab and returning.

## Scripted Checks

- Reliability harness: [amuxd/scripts/terminal_reliability_5000_keys.sh](/home/k/code/amux/amuxd/scripts/terminal_reliability_5000_keys.sh)
- Latency harness: [amuxd/scripts/measure_terminal_latency.sh](/home/k/code/amux/amuxd/scripts/measure_terminal_latency.sh)
- Renderer benchmark: [amuxterm-web/scripts/benchmark_renderer.sh](/home/k/code/amux/amuxterm-web/scripts/benchmark_renderer.sh)

## Interoperability Notes

- iOS Safari: pay attention to virtual keyboard resize timing and focus restoration after paste.
- Android Chrome: verify modifier latches survive keyboard open/close transitions.
- Desktop Chromium: validate clipboard parity with mobile and fixture correctness.
- Desktop Firefox: validate text metrics and repaint behavior against the same fixture set.
