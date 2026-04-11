# terminal-web-surface-v1 Specification

## Purpose
TBD - created by archiving change terminal-renderer-v1-baseline. Update Purpose after archive.
## Requirements
### Requirement: Rust-first terminal core baseline
The system SHALL implement the v1 web terminal core using `vte` for escape parsing and `vt100` for terminal state, with width and grapheme handling via `unicode-width` and `unicode-segmentation`.

#### Scenario: Baseline stack used in v1 terminal path
- **WHEN** the v1 web terminal processes terminal output
- **THEN** escape parsing uses `vte` and terminal state updates use `vt100`
- **AND** character width and grapheme segmentation use `unicode-width` and `unicode-segmentation`

#### Scenario: Escalation to alternate state core
- **WHEN** the baseline stack fails fidelity acceptance for two consecutive milestones on IME, wrapping, or ANSI edge fixtures
- **THEN** the system evaluates migrating terminal state handling to `alacritty_terminal` behind the same client-facing contract

### Requirement: Backend-agnostic terminal interaction contract
The system SHALL expose terminal interaction semantics that do not require tmux-specific payload fields or command knowledge.

#### Scenario: Client consumes terminal session without tmux details
- **WHEN** a client renders output and sends input for an active terminal session
- **THEN** the client can do so without using tmux pane identifiers, tmux command names, or tmux-specific transport fields

### Requirement: Mobile modifier input baseline
The system SHALL support mobile terminal interaction on iOS Safari and Android Chrome with explicit modifier-driven key entry.

#### Scenario: Required key set is available on touch clients
- **WHEN** a user interacts with the terminal on iOS Safari or Android Chrome
- **THEN** the user can input text plus `Ctrl`, `Esc`, `Tab`, arrow keys, and `Enter` through the mobile modifier UX

#### Scenario: Orientation and paste stability
- **WHEN** a mobile user performs copy/paste actions or changes device orientation during an active session
- **THEN** terminal state remains coherent with no cursor drift or output corruption

### Requirement: Mobile input reliability threshold
The system SHALL achieve a minimum input reliability threshold for the required key set in mobile browsers.

#### Scenario: Scripted key reliability run
- **WHEN** a 5,000-key scripted input run is executed on iOS Safari or Android Chrome
- **THEN** at least 99.9% of intended key events are delivered to the terminal session

### Requirement: Browser support priority baseline
The system SHALL prioritize browser interoperability work in a fixed v1 order.

#### Scenario: Quirk triage order
- **WHEN** terminal interoperability issues are triaged for v1
- **THEN** the implementation order prioritizes iOS Safari, then Android Chrome, then desktop Chromium, then desktop Firefox
- **AND** issue classes are prioritized as IME/virtual keyboard, canvas metrics and DPR scaling, clipboard/selection, then focus restoration

### Requirement: Terminal latency budget
The system SHALL meet explicit keypress-to-echo latency budgets for active sessions.

#### Scenario: Local and LAN latency budget
- **WHEN** keypress-to-visible-echo latency is measured on local or LAN links
- **THEN** p95 latency is less than or equal to 160 ms

#### Scenario: Typical remote latency budget
- **WHEN** keypress-to-visible-echo latency is measured on typical remote links
- **THEN** p95 latency is less than or equal to 280 ms

### Requirement: Rendering and throughput budgets
The system SHALL meet explicit rendering and update-throughput budgets per active terminal session.

#### Scenario: Frame-time budget under sustained output
- **WHEN** the web terminal renders sustained output at the v1 target load
- **THEN** p95 frame time is less than or equal to 16 ms

#### Scenario: Cell update throughput budget
- **WHEN** an active session produces terminal updates at 2,000 cell updates per second
- **THEN** the client continues rendering without dropped lifecycle events

