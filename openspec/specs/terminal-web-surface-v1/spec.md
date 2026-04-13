## MODIFIED Requirements

### Requirement: Rust-first terminal core baseline

The system SHALL implement the v1 web terminal surface with `ghostty-web` as the browser-side terminal engine, while keeping the client-facing terminal interaction contract backend-agnostic.

#### Scenario: Ghostty-web baseline used in v1 terminal path

- **WHEN** the v1 web terminal processes live terminal output in the browser
- **THEN** browser-side terminal emulation is handled by `ghostty-web`
- **AND** the daemon no longer depends on `vte`, `vt100`, `unicode-width`, or `unicode-segmentation` for live rendering

## REMOVED Requirements

### Requirement: Mobile modifier input baseline

**Reason**: The detailed mobile modifier UX now lives in `mobile-terminal-input`, but the broader browser, latency, and throughput requirements in this capability remain in force.
