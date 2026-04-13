use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalStack {
    pub escape_parser: String,
    pub state_core: TerminalStateCore,
    pub width_engine: String,
    pub grapheme_engine: String,
}

impl TerminalStack {
    pub fn baseline() -> Self {
        Self {
            escape_parser: "tmux-capture".to_string(),
            state_core: TerminalStateCore::TmuxCapture,
            width_engine: "browser".to_string(),
            grapheme_engine: "browser".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TerminalStateCore {
    TmuxCapture,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalFallbackPolicy {
    pub alternate_state_core: TerminalStateCore,
    pub tracked_failure_classes: Vec<String>,
    pub consecutive_milestones_required: u8,
}

impl TerminalFallbackPolicy {
    pub fn baseline() -> Self {
        Self {
            alternate_state_core: TerminalStateCore::TmuxCapture,
            tracked_failure_classes: Vec::new(),
            consecutive_milestones_required: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalInputCapabilities {
    pub text: bool,
    pub paste: bool,
    pub resize: bool,
    pub ctrl_character_chords: bool,
    pub alt_prefix: bool,
    pub named_keys: Vec<TerminalNamedKey>,
}

impl TerminalInputCapabilities {
    pub fn baseline() -> Self {
        Self {
            text: true,
            paste: true,
            resize: true,
            ctrl_character_chords: true,
            alt_prefix: true,
            named_keys: vec![
                TerminalNamedKey::Ctrl,
                TerminalNamedKey::Escape,
                TerminalNamedKey::Tab,
                TerminalNamedKey::ArrowUp,
                TerminalNamedKey::ArrowDown,
                TerminalNamedKey::ArrowLeft,
                TerminalNamedKey::ArrowRight,
                TerminalNamedKey::Enter,
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalSurfaceState {
    pub session_id: String,
    pub stack: TerminalStack,
    pub fallback_policy: TerminalFallbackPolicy,
    pub input_capabilities: TerminalInputCapabilities,
    pub snapshot: TerminalSnapshot,
}

impl TerminalSurfaceState {
    pub fn baseline(session_id: String, snapshot: TerminalSnapshot) -> Self {
        Self {
            session_id,
            stack: TerminalStack::baseline(),
            fallback_policy: TerminalFallbackPolicy::baseline(),
            input_capabilities: TerminalInputCapabilities::baseline(),
            snapshot,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalSnapshot {
    pub rows: u16,
    pub cols: u16,
    pub cursor: TerminalCursor,
    pub modes: TerminalModes,
    pub escape_sequence_metrics: EscapeSequenceMetrics,
    pub lines: Vec<TerminalLine>,
    pub scrollback: Vec<TerminalLine>,
    pub plain_text: String,
}

impl TerminalSnapshot {
    pub fn with_scrollback(mut self, scrollback: Vec<TerminalLine>, plain_text: String) -> Self {
        self.scrollback = scrollback;
        self.plain_text = plain_text;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalCursor {
    pub row: u16,
    pub col: u16,
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalModes {
    pub application_cursor: bool,
    pub application_keypad: bool,
    pub bracketed_paste: bool,
    pub alternate_screen: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EscapeSequenceMetrics {
    pub print: usize,
    pub execute: usize,
    pub csi: usize,
    pub esc: usize,
    pub osc: usize,
    pub dcs: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalLine {
    pub row: u16,
    pub wrapped: bool,
    pub cells: Vec<TerminalCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalCell {
    pub column: u16,
    pub text: String,
    pub column_span: u8,
    pub unicode_width: u8,
    pub grapheme_count: u8,
    pub is_wide: bool,
    pub is_wide_continuation: bool,
    pub foreground: TerminalColor,
    pub background: TerminalColor,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum TerminalColor {
    Default,
    Indexed(u8),
    Rgb([u8; 3]),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalInputRequest {
    pub events: Vec<TerminalInputEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TerminalInputEvent {
    Text {
        text: String,
    },
    Paste {
        text: String,
    },
    Key {
        key: TerminalKey,
        ctrl: bool,
        alt: bool,
        shift: bool,
    },
    Resize {
        rows: u16,
        cols: u16,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TerminalKey {
    Named { key: TerminalNamedKey },
    Character { text: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TerminalNamedKey {
    Ctrl,
    Escape,
    Tab,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Enter,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalInputResponse {
    pub accepted_events: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TerminalControlMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub rows: Option<u16>,
    pub cols: Option<u16>,
}
