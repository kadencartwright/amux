use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation as _;
use unicode_width::UnicodeWidthStr as _;

const BASELINE_ESCAPE_PARSER: &str = "vte";
const BASELINE_STATE_CORE: TerminalStateCore = TerminalStateCore::Vt100;
const BASELINE_WIDTH_ENGINE: &str = "unicode-width";
const BASELINE_GRAPHEME_ENGINE: &str = "unicode-segmentation";

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
            escape_parser: BASELINE_ESCAPE_PARSER.to_string(),
            state_core: BASELINE_STATE_CORE,
            width_engine: BASELINE_WIDTH_ENGINE.to_string(),
            grapheme_engine: BASELINE_GRAPHEME_ENGINE.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TerminalStateCore {
    Vt100,
    AlacrittyTerminal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalFallbackPolicy {
    pub alternate_state_core: TerminalStateCore,
    pub tracked_failure_classes: Vec<TerminalFidelityClass>,
    pub consecutive_milestones_required: u8,
}

impl TerminalFallbackPolicy {
    pub fn baseline() -> Self {
        Self {
            alternate_state_core: TerminalStateCore::AlacrittyTerminal,
            tracked_failure_classes: vec![
                TerminalFidelityClass::Ime,
                TerminalFidelityClass::Wrapping,
                TerminalFidelityClass::AnsiEdgeFixtures,
            ],
            consecutive_milestones_required: 2,
        }
    }

    pub fn should_evaluate_alternate_state_core(
        &self,
        milestones: &[MilestoneFidelityResult],
    ) -> bool {
        if usize::from(self.consecutive_milestones_required) > milestones.len() {
            return false;
        }

        milestones.windows(2).any(|pair| {
            pair[0].failed_classes.iter().any(|class| {
                self.tracked_failure_classes.contains(class)
                    && pair[1].failed_classes.contains(class)
            })
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TerminalFidelityClass {
    Ime,
    Wrapping,
    AnsiEdgeFixtures,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MilestoneFidelityResult {
    pub milestone: String,
    pub failed_classes: Vec<TerminalFidelityClass>,
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

    pub fn diff_frame(
        &self,
        session_id: &str,
        sequence: u64,
        previous: Option<&Self>,
    ) -> TerminalStreamFrame {
        let lines = match previous {
            None => self.lines.clone(),
            Some(previous) if previous.rows != self.rows || previous.cols != self.cols => {
                self.lines.clone()
            }
            Some(previous) => self
                .lines
                .iter()
                .filter(|line| {
                    previous
                        .lines
                        .iter()
                        .find(|candidate| candidate.row == line.row)
                        != Some(*line)
                })
                .cloned()
                .collect(),
        };

        TerminalStreamFrame {
            session_id: session_id.to_string(),
            sequence,
            rows: self.rows,
            cols: self.cols,
            cursor: self.cursor.clone(),
            modes: self.modes.clone(),
            escape_sequence_metrics: self.escape_sequence_metrics.clone(),
            lines,
        }
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

#[derive(Default)]
struct EscapeSequenceTracker {
    metrics: EscapeSequenceMetrics,
}

impl vte::Perform for EscapeSequenceTracker {
    fn print(&mut self, _c: char) {
        self.metrics.print += 1;
    }

    fn execute(&mut self, _byte: u8) {
        self.metrics.execute += 1;
    }

    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {
        self.metrics.dcs += 1;
    }

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {
        self.metrics.osc += 1;
    }

    fn csi_dispatch(
        &mut self,
        _params: &vte::Params,
        _intermediates: &[u8],
        _ignore: bool,
        _action: char,
    ) {
        self.metrics.csi += 1;
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {
        self.metrics.esc += 1;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalLine {
    pub row: u16,
    pub wrapped: bool,
    pub cells: Vec<TerminalCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalStreamFrame {
    pub session_id: String,
    pub sequence: u64,
    pub rows: u16,
    pub cols: u16,
    pub cursor: TerminalCursor,
    pub modes: TerminalModes,
    pub escape_sequence_metrics: EscapeSequenceMetrics,
    pub lines: Vec<TerminalLine>,
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

pub struct TerminalCore {
    escape_parser: vte::Parser,
    escape_tracker: EscapeSequenceTracker,
    state_parser: vt100::Parser,
}

impl TerminalCore {
    pub fn new(rows: u16, cols: u16, scrollback_len: usize) -> Self {
        Self {
            escape_parser: vte::Parser::new(),
            escape_tracker: EscapeSequenceTracker::default(),
            state_parser: vt100::Parser::new(rows, cols, scrollback_len),
        }
    }

    pub fn ingest(&mut self, bytes: &[u8]) {
        self.escape_parser.advance(&mut self.escape_tracker, bytes);
        self.state_parser.process(bytes);
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.state_parser.screen_mut().set_size(rows, cols);
    }

    pub fn snapshot(&self) -> TerminalSnapshot {
        let screen = self.state_parser.screen();
        let (rows, cols) = screen.size();
        let (cursor_row, cursor_col) = screen.cursor_position();
        let lines = (0..rows)
            .map(|row| TerminalLine {
                row,
                wrapped: screen.row_wrapped(row),
                cells: (0..cols)
                    .map(|column| map_cell(column, screen.cell(row, column)))
                    .collect(),
            })
            .collect();

        TerminalSnapshot {
            rows,
            cols,
            cursor: TerminalCursor {
                row: cursor_row,
                col: cursor_col,
                visible: !screen.hide_cursor(),
            },
            modes: TerminalModes {
                application_cursor: screen.application_cursor(),
                application_keypad: screen.application_keypad(),
                bracketed_paste: screen.bracketed_paste(),
                alternate_screen: screen.alternate_screen(),
            },
            escape_sequence_metrics: self.escape_tracker.metrics.clone(),
            lines,
            scrollback: Vec::new(),
            plain_text: screen.contents(),
        }
    }
}

fn map_cell(column: u16, cell: Option<&vt100::Cell>) -> TerminalCell {
    let Some(cell) = cell else {
        return TerminalCell {
            column,
            text: String::new(),
            column_span: 1,
            unicode_width: 0,
            grapheme_count: 0,
            is_wide: false,
            is_wide_continuation: false,
            foreground: TerminalColor::Default,
            background: TerminalColor::Default,
            bold: false,
            italic: false,
            underline: false,
            inverse: false,
        };
    };

    let text = cell.contents().to_string();
    let column_span = if cell.is_wide_continuation() {
        0
    } else if cell.is_wide() {
        2
    } else {
        1
    };
    let unicode_width = u8::try_from(text.width()).unwrap_or(u8::MAX);
    let grapheme_count = u8::try_from(text.graphemes(true).count()).unwrap_or(u8::MAX);

    TerminalCell {
        column,
        text,
        column_span,
        unicode_width,
        grapheme_count,
        is_wide: cell.is_wide(),
        is_wide_continuation: cell.is_wide_continuation(),
        foreground: map_color(cell.fgcolor()),
        background: map_color(cell.bgcolor()),
        bold: cell.bold(),
        italic: cell.italic(),
        underline: cell.underline(),
        inverse: cell.inverse(),
    }
}

fn map_color(color: vt100::Color) -> TerminalColor {
    match color {
        vt100::Color::Default => TerminalColor::Default,
        vt100::Color::Idx(idx) => TerminalColor::Indexed(idx),
        vt100::Color::Rgb(r, g, b) => TerminalColor::Rgb([r, g, b]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_core_emits_baseline_snapshot() {
        let mut core = TerminalCore::new(3, 8, 0);
        core.ingest(b"ab\x1b[31mc\x1b[m\r\n\xF0\x9F\x98\x80");
        let snapshot = core.snapshot();

        assert_eq!(TerminalStack::baseline().escape_parser, "vte");
        assert_eq!(
            TerminalStack::baseline().state_core,
            TerminalStateCore::Vt100
        );
        assert_eq!(snapshot.rows, 3);
        assert_eq!(snapshot.cols, 8);
        assert_eq!(snapshot.escape_sequence_metrics.csi, 2);
        assert_eq!(
            snapshot.lines[0].cells[2].foreground,
            TerminalColor::Indexed(1)
        );
        assert_eq!(snapshot.lines[1].cells[0].text, "😀");
        assert!(snapshot.lines[1].cells[0].unicode_width >= 2);
        assert_eq!(snapshot.lines[1].cells[0].grapheme_count, 1);
    }

    #[test]
    fn fallback_policy_requires_two_consecutive_milestones() {
        let policy = TerminalFallbackPolicy::baseline();
        let milestones = vec![
            MilestoneFidelityResult {
                milestone: "m3".to_string(),
                failed_classes: vec![TerminalFidelityClass::Ime],
            },
            MilestoneFidelityResult {
                milestone: "m4".to_string(),
                failed_classes: vec![TerminalFidelityClass::Ime],
            },
        ];

        assert!(policy.should_evaluate_alternate_state_core(&milestones));
    }

    #[test]
    fn fallback_policy_ignores_non_consecutive_failures() {
        let policy = TerminalFallbackPolicy::baseline();
        let milestones = vec![
            MilestoneFidelityResult {
                milestone: "m3".to_string(),
                failed_classes: vec![TerminalFidelityClass::Wrapping],
            },
            MilestoneFidelityResult {
                milestone: "m4".to_string(),
                failed_classes: Vec::new(),
            },
            MilestoneFidelityResult {
                milestone: "m5".to_string(),
                failed_classes: vec![TerminalFidelityClass::Wrapping],
            },
        ];

        assert!(!policy.should_evaluate_alternate_state_core(&milestones));
    }
}
