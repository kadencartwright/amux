**DEPRECATED**: This capability is deprecated in favor of ghostty-terminal-stream and mobile-terminal-input.

Rendering is now handled entirely by the client-side ghostty-web library, which provides:

- Full VT100 emulation via Ghostty's battle-tested parser
- Cursor rendering, text selection, and copy/paste
- ANSI escape sequence support (including XTPUSHSGR/XTPOPSGR)
- Mobile touch support

The backend no longer serializes terminal state. The client receives raw PTY bytes and renders them locally.

## DEPRECATED Requirements

### Requirement: Rust-first terminal core baseline

**Reason**: Backend no longer handles terminal parsing. Client (ghostty-web) provides terminal emulation.

### Requirement: Backend-agnostic terminal interaction contract

**Reason**: Still valid - tmux details remain hidden from client. But implementation changed.

### Requirement: Mobile modifier input baseline

**Reason**: Replaced by mobile-terminal-input capability with unified keyboard area.

### Requirement: Mobile input reliability threshold

**Reason**: Replaced by mobile-terminal-input capability.

### Requirement: Browser support priority baseline

**Reason**: Browser quirks are now handled by ghostty-web.

### Requirement: Terminal latency budget

**Reason**: Still applicable but measured differently. ghostty-web handles rendering locally.

### Requirement: Rendering and throughput budgets

**Reason**: ghostty-web handles rendering. Performance budgets apply to the WASM renderer internally.
