# AMUX

AMUX provides a tmux-backed session daemon (`amuxd`) and a browser shell (`amuxshell-web`) for interactive terminal workflows.

## Terminal Architecture

- `GET /sessions/{id}/terminal` provides bootstrap and resync snapshots for selected sessions.
- `GET /sessions/{id}/terminal/stream` provides bidirectional WebSocket transport.
  - Binary frames carry raw PTY bytes in both directions.
  - Text frames carry JSON control messages (for example: resize).
- Browser rendering is handled by `ghostty-web` in `amuxshell-web`.
- Legacy `POST /sessions/{id}/terminal/input` can be retained behind `AMUXD_TERMINAL_HTTP_INPUT_MIGRATION=1` during rollout.

## Build

- Shell assets: `cd amuxshell-web && npm run build`
- Daemon: `cd amuxd && cargo build`
