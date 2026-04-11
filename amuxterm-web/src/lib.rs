use std::collections::BTreeSet;
use std::time::Instant;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalSurfaceState {
    pub session_id: String,
    pub snapshot: TerminalSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalSnapshot {
    pub rows: u16,
    pub cols: u16,
    pub cursor: TerminalCursor,
    pub lines: Vec<TerminalLine>,
    pub plain_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalCursor {
    pub row: u16,
    pub col: u16,
    pub visible: bool,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Orientation {
    Portrait,
    Landscape,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct CellMetrics {
    pub width_px: f32,
    pub height_px: f32,
    pub baseline_px: f32,
}

impl Default for CellMetrics {
    fn default() -> Self {
        Self {
            width_px: 9.0,
            height_px: 18.0,
            baseline_px: 14.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ViewportState {
    pub width_px: f32,
    pub height_px: f32,
    pub device_pixel_ratio: f32,
    pub orientation: Orientation,
    pub layout_epoch: u64,
}

impl Default for ViewportState {
    fn default() -> Self {
        Self {
            width_px: 0.0,
            height_px: 0.0,
            device_pixel_ratio: 1.0,
            orientation: Orientation::Unknown,
            layout_epoch: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenderReport {
    pub frame_index: u64,
    pub full_repaint: bool,
    pub dirty_cells: usize,
    pub draw_operations: usize,
    pub layout_epoch: u64,
}

const DEFAULT_FOREGROUND_COLOR: &str = "#e5e7eb";
const DEFAULT_BACKGROUND_COLOR: &str = "#282c34";

pub trait CanvasPainter {
    fn begin_frame(&mut self, width_px: f32, height_px: f32);
    fn clear(&mut self);
    fn draw_cell(&mut self, x_px: f32, y_px: f32, metrics: CellMetrics, cell: &TerminalCell);
    fn finish_frame(&mut self);
}

#[derive(Debug, Default, Clone)]
pub struct NoopCanvasPainter {
    pub frames_started: usize,
    pub cells_drawn: usize,
    pub last_canvas_size: Option<(f32, f32)>,
}

impl CanvasPainter for NoopCanvasPainter {
    fn begin_frame(&mut self, width_px: f32, height_px: f32) {
        self.frames_started += 1;
        self.last_canvas_size = Some((width_px, height_px));
    }

    fn clear(&mut self) {}

    fn draw_cell(&mut self, _x_px: f32, _y_px: f32, _metrics: CellMetrics, cell: &TerminalCell) {
        if !cell.text.is_empty() {
            self.cells_drawn += 1;
        }
    }

    fn finish_frame(&mut self) {}
}

pub struct TerminalCanvasRenderer<P> {
    painter: P,
    metrics: CellMetrics,
    viewport: ViewportState,
    previous: Option<TerminalSnapshot>,
    frame_index: u64,
    force_full_repaint: bool,
}

impl<P> TerminalCanvasRenderer<P>
where
    P: CanvasPainter,
{
    pub fn new(painter: P, metrics: CellMetrics) -> Self {
        Self {
            painter,
            metrics,
            viewport: ViewportState::default(),
            previous: None,
            frame_index: 0,
            force_full_repaint: true,
        }
    }

    pub fn painter(&self) -> &P {
        &self.painter
    }

    pub fn painter_mut(&mut self) -> &mut P {
        &mut self.painter
    }

    pub fn viewport(&self) -> &ViewportState {
        &self.viewport
    }

    pub fn handle_viewport_change(
        &mut self,
        width_px: f32,
        height_px: f32,
        device_pixel_ratio: f32,
        orientation: Orientation,
    ) {
        let changed = self.viewport.width_px != width_px
            || self.viewport.height_px != height_px
            || (self.viewport.device_pixel_ratio - device_pixel_ratio).abs() > f32::EPSILON
            || self.viewport.orientation != orientation;

        if changed {
            self.viewport = ViewportState {
                width_px,
                height_px,
                device_pixel_ratio,
                orientation,
                layout_epoch: self.viewport.layout_epoch + 1,
            };
            self.force_full_repaint = true;
        }
    }

    pub fn render(&mut self, surface: &TerminalSurfaceState) -> RenderReport {
        let width_px = self.viewport_width(surface.snapshot.cols);
        let height_px = self.viewport_height(surface.snapshot.rows);
        let previous = self.previous.as_ref();
        let full_repaint = self.force_full_repaint
            || previous.is_none()
            || previous.is_some_and(|snapshot| {
                snapshot.rows != surface.snapshot.rows || snapshot.cols != surface.snapshot.cols
            });

        self.painter.begin_frame(width_px, height_px);
        if full_repaint {
            self.painter.clear();
        }

        let mut dirty_cells = 0;
        let mut draw_operations = 0;
        for line in &surface.snapshot.lines {
            for cell in &line.cells {
                if full_repaint || cell_changed(previous, line.row, cell) {
                    let x_px = f32::from(cell.column) * self.metrics.width_px;
                    let y_px = f32::from(line.row) * self.metrics.height_px;
                    self.painter.draw_cell(x_px, y_px, self.metrics, cell);
                    dirty_cells += 1;
                    draw_operations += 1;
                }
            }
        }

        self.painter.finish_frame();
        self.previous = Some(surface.snapshot.clone());
        self.force_full_repaint = false;
        self.frame_index += 1;

        RenderReport {
            frame_index: self.frame_index,
            full_repaint,
            dirty_cells,
            draw_operations,
            layout_epoch: self.viewport.layout_epoch,
        }
    }

    fn viewport_width(&self, cols: u16) -> f32 {
        let fallback = f32::from(cols) * self.metrics.width_px;
        if self.viewport.width_px > 0.0 {
            self.viewport.width_px
        } else {
            fallback
        }
    }

    fn viewport_height(&self, rows: u16) -> f32 {
        let fallback = f32::from(rows) * self.metrics.height_px;
        if self.viewport.height_px > 0.0 {
            self.viewport.height_px
        } else {
            fallback
        }
    }
}

fn cell_changed(previous: Option<&TerminalSnapshot>, row: u16, cell: &TerminalCell) -> bool {
    let Some(previous) = previous else {
        return true;
    };
    let Some(previous_line) = previous.lines.iter().find(|line| line.row == row) else {
        return true;
    };
    let Some(previous_cell) = previous_line
        .cells
        .iter()
        .find(|candidate| candidate.column == cell.column)
    else {
        return true;
    };

    previous_cell != cell
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModifierLatch {
    Ctrl,
    Alt,
    Shift,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MobileInputState {
    latched_modifiers: BTreeSet<ModifierLatch>,
    selection_anchor: Option<(u16, u16)>,
    pending_paste: Option<String>,
    composition_buffer: String,
    viewport_epoch: u64,
}

impl MobileInputState {
    pub fn toggle_modifier(&mut self, modifier: ModifierLatch) {
        if !self.latched_modifiers.remove(&modifier) {
            self.latched_modifiers.insert(modifier);
        }
    }

    pub fn latched_modifiers(&self) -> &BTreeSet<ModifierLatch> {
        &self.latched_modifiers
    }

    pub fn set_selection_anchor(&mut self, row: u16, col: u16) {
        self.selection_anchor = Some((row, col));
    }

    pub fn start_composition(&mut self, composition: impl Into<String>) {
        self.composition_buffer = composition.into();
    }

    pub fn emit_text(&mut self, text: impl Into<String>) -> TerminalInputEvent {
        let text = text.into();
        let ctrl = self.consume_modifier(ModifierLatch::Ctrl);
        let alt = self.consume_modifier(ModifierLatch::Alt);
        let shift = self.consume_modifier(ModifierLatch::Shift);

        if ctrl && text.chars().count() == 1 {
            TerminalInputEvent::Key {
                key: TerminalKey::Character { text },
                ctrl,
                alt,
                shift,
            }
        } else {
            TerminalInputEvent::Text { text }
        }
    }

    pub fn emit_named_key(&mut self, key: TerminalNamedKey) -> TerminalInputEvent {
        let ctrl = self.consume_modifier(ModifierLatch::Ctrl);
        let alt = self.consume_modifier(ModifierLatch::Alt);
        let shift = self.consume_modifier(ModifierLatch::Shift);

        TerminalInputEvent::Key {
            key: TerminalKey::Named { key },
            ctrl,
            alt,
            shift,
        }
    }

    pub fn emit_paste(&mut self, text: impl Into<String>) -> TerminalInputEvent {
        let text = text.into();
        self.pending_paste = Some(text.clone());
        self.latched_modifiers.clear();
        TerminalInputEvent::Paste { text }
    }

    pub fn mark_paste_committed(&mut self) {
        self.pending_paste = None;
    }

    pub fn handle_viewport_change(&mut self, viewport: &ViewportState) {
        if viewport.layout_epoch == self.viewport_epoch {
            return;
        }

        self.viewport_epoch = viewport.layout_epoch;
        self.selection_anchor = None;
        self.pending_paste = None;
        self.composition_buffer.clear();
        self.latched_modifiers.clear();
    }

    pub fn pending_paste(&self) -> Option<&str> {
        self.pending_paste.as_deref()
    }

    pub fn composition_buffer(&self) -> &str {
        &self.composition_buffer
    }

    fn consume_modifier(&mut self, modifier: ModifierLatch) -> bool {
        self.latched_modifiers.remove(&modifier)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenderBenchmarkSummary {
    pub frames: usize,
    pub rows: u16,
    pub cols: u16,
    pub updates_per_frame: usize,
    pub p95_frame_time_ms: f64,
    pub max_frame_time_ms: f64,
    pub mean_frame_time_ms: f64,
}

pub fn benchmark_renderer(
    frames: usize,
    rows: u16,
    cols: u16,
    updates_per_frame: usize,
) -> RenderBenchmarkSummary {
    let painter = NoopCanvasPainter::default();
    let mut renderer = TerminalCanvasRenderer::new(painter, CellMetrics::default());
    let mut surface = sample_surface(rows, cols);
    let mut timings = Vec::with_capacity(frames);

    for frame in 0..frames {
        mutate_surface(&mut surface, frame, updates_per_frame);
        let started = Instant::now();
        let _ = renderer.render(&surface);
        timings.push(started.elapsed().as_secs_f64() * 1000.0);
    }

    let p95_frame_time_ms = percentile_ms(&timings, 0.95);
    let max_frame_time_ms = timings.iter().copied().fold(0.0_f64, f64::max);
    let mean_frame_time_ms = if timings.is_empty() {
        0.0
    } else {
        timings.iter().sum::<f64>() / timings.len() as f64
    };

    RenderBenchmarkSummary {
        frames,
        rows,
        cols,
        updates_per_frame,
        p95_frame_time_ms,
        max_frame_time_ms,
        mean_frame_time_ms,
    }
}

fn percentile_ms(samples: &[f64], quantile: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }

    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let idx = ((sorted.len() - 1) as f64 * quantile).round() as usize;
    sorted[idx]
}

fn sample_surface(rows: u16, cols: u16) -> TerminalSurfaceState {
    let lines = (0..rows)
        .map(|row| TerminalLine {
            row,
            wrapped: false,
            cells: (0..cols)
                .map(|column| TerminalCell {
                    column,
                    text: " ".to_string(),
                    foreground: TerminalColor::Default,
                    background: TerminalColor::Default,
                    bold: false,
                    italic: false,
                    underline: false,
                    inverse: false,
                })
                .collect(),
        })
        .collect();

    TerminalSurfaceState {
        session_id: "benchmark".to_string(),
        snapshot: TerminalSnapshot {
            rows,
            cols,
            cursor: TerminalCursor {
                row: 0,
                col: 0,
                visible: true,
            },
            lines,
            plain_text: String::new(),
        },
    }
}

fn mutate_surface(surface: &mut TerminalSurfaceState, frame: usize, updates_per_frame: usize) {
    let rows = usize::from(surface.snapshot.rows);
    let cols = usize::from(surface.snapshot.cols);
    if rows == 0 || cols == 0 {
        return;
    }

    for idx in 0..updates_per_frame {
        let linear = (frame * updates_per_frame + idx) % (rows * cols);
        let row = linear / cols;
        let column = linear % cols;
        let cell = &mut surface.snapshot.lines[row].cells[column];
        let ch = ((linear + frame) % 26) as u8 + b'a';
        cell.text = char::from(ch).to_string();
        cell.bold = frame % 2 == 0;
        cell.underline = frame % 3 == 0;
    }
}

fn resolve_web_cell_colors(cell: &TerminalCell) -> (String, String) {
    let mut foreground = web_color(&cell.foreground, false);
    let mut background = web_color(&cell.background, true);

    if cell.inverse {
        std::mem::swap(&mut foreground, &mut background);
    }

    (foreground, background)
}

fn web_color(color: &TerminalColor, background: bool) -> String {
    match color {
        TerminalColor::Default => {
            if background {
                DEFAULT_BACKGROUND_COLOR.to_string()
            } else {
                DEFAULT_FOREGROUND_COLOR.to_string()
            }
        }
        TerminalColor::Indexed(idx) => indexed_web_color(*idx).to_string(),
        TerminalColor::Rgb([r, g, b]) => format!("rgb({r}, {g}, {b})"),
    }
}

fn indexed_web_color(index: u8) -> &'static str {
    match index {
        0 => "#1f2430",
        1 => "#ff6b6b",
        2 => "#98c379",
        3 => "#e5c07b",
        4 => "#61afef",
        5 => "#c678dd",
        6 => "#56b6c2",
        7 => "#dcdfe4",
        8 => "#5c6370",
        9 => "#ff7b72",
        10 => "#b8e994",
        11 => "#f4d35e",
        12 => "#7cc7ff",
        13 => "#d2a8ff",
        14 => "#7ee7f2",
        _ => "#f8fafc",
    }
}

#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
pub mod wasm {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::*;
    use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

    use super::{
        CellMetrics, Orientation, RenderReport, TerminalCanvasRenderer, TerminalCell,
        TerminalSurfaceState, resolve_web_cell_colors,
    };

    struct WebCanvasPainter {
        context: CanvasRenderingContext2d,
    }

    impl super::CanvasPainter for WebCanvasPainter {
        fn begin_frame(&mut self, width_px: f32, height_px: f32) {
            if let Some(canvas) = self.context.canvas() {
                let width = width_px.ceil() as u32;
                let height = height_px.ceil() as u32;
                if canvas.width() != width || canvas.height() != height {
                    canvas.set_width(width);
                    canvas.set_height(height);
                    self.context.set_font("14px monospace");
                    self.context.set_text_baseline("alphabetic");
                }
            }
        }

        fn clear(&mut self) {
            if let Some(canvas) = self.context.canvas() {
                self.context.clear_rect(
                    0.0,
                    0.0,
                    f64::from(canvas.width()),
                    f64::from(canvas.height()),
                );
            }
        }

        fn draw_cell(&mut self, x_px: f32, y_px: f32, metrics: CellMetrics, cell: &TerminalCell) {
            let (fg, bg) = resolve_web_cell_colors(cell);
            self.context.set_fill_style_str(&bg);
            self.context.fill_rect(
                f64::from(x_px),
                f64::from(y_px),
                f64::from(metrics.width_px),
                f64::from(metrics.height_px),
            );

            if cell.text.is_empty() {
                return;
            }

            self.context.set_fill_style_str(&fg);
            let _ = self.context.fill_text(
                &cell.text,
                f64::from(x_px),
                f64::from(y_px + metrics.baseline_px),
            );
        }

        fn finish_frame(&mut self) {}
    }

    #[wasm_bindgen]
    pub struct WasmCanvasRenderer {
        inner: TerminalCanvasRenderer<WebCanvasPainter>,
    }

    #[wasm_bindgen]
    impl WasmCanvasRenderer {
        #[wasm_bindgen(constructor)]
        pub fn new(canvas: HtmlCanvasElement) -> Result<WasmCanvasRenderer, JsValue> {
            let context = canvas
                .get_context("2d")?
                .ok_or_else(|| JsValue::from_str("2d context unavailable"))?
                .dyn_into::<CanvasRenderingContext2d>()?;
            context.set_font("14px monospace");

            Ok(WasmCanvasRenderer {
                inner: TerminalCanvasRenderer::new(
                    WebCanvasPainter { context },
                    CellMetrics::default(),
                ),
            })
        }

        pub fn handle_viewport_change(
            &mut self,
            width_px: f32,
            height_px: f32,
            device_pixel_ratio: f32,
            orientation: &str,
        ) {
            let orientation = match orientation {
                "portrait" => Orientation::Portrait,
                "landscape" => Orientation::Landscape,
                _ => Orientation::Unknown,
            };
            self.inner
                .handle_viewport_change(width_px, height_px, device_pixel_ratio, orientation);
        }

        pub fn render_surface_json(&mut self, surface_json: &str) -> Result<JsValue, JsValue> {
            let surface: TerminalSurfaceState = serde_json::from_str(surface_json)
                .map_err(|err| JsValue::from_str(&err.to_string()))?;
            let report: RenderReport = self.inner.render(&surface);
            serde_wasm_bindgen::to_value(&report).map_err(|err| JsValue::from_str(&err.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_loop_repaints_diffs_after_initial_frame() {
        let mut renderer =
            TerminalCanvasRenderer::new(NoopCanvasPainter::default(), CellMetrics::default());
        let mut surface = sample_surface(2, 3);

        let first = renderer.render(&surface);
        assert!(first.full_repaint);
        assert_eq!(first.dirty_cells, 6);

        surface.snapshot.lines[0].cells[1].text = "x".to_string();
        let second = renderer.render(&surface);
        assert!(!second.full_repaint);
        assert_eq!(second.dirty_cells, 1);
    }

    #[test]
    fn mobile_modifier_mapping_supports_required_key_set() {
        let mut mobile = MobileInputState::default();

        mobile.toggle_modifier(ModifierLatch::Ctrl);
        let ctrl_c = mobile.emit_text("c");
        assert_eq!(
            ctrl_c,
            TerminalInputEvent::Key {
                key: TerminalKey::Character {
                    text: "c".to_string(),
                },
                ctrl: true,
                alt: false,
                shift: false,
            }
        );

        let enter = mobile.emit_named_key(TerminalNamedKey::Enter);
        assert_eq!(
            enter,
            TerminalInputEvent::Key {
                key: TerminalKey::Named {
                    key: TerminalNamedKey::Enter,
                },
                ctrl: false,
                alt: false,
                shift: false,
            }
        );
    }

    #[test]
    fn viewport_change_clears_paste_and_composition_state() {
        let mut renderer =
            TerminalCanvasRenderer::new(NoopCanvasPainter::default(), CellMetrics::default());
        let mut mobile = MobileInputState::default();
        mobile.toggle_modifier(ModifierLatch::Ctrl);
        mobile.set_selection_anchor(4, 8);
        mobile.start_composition("ime");
        let paste = mobile.emit_paste("clipboard");
        assert_eq!(
            paste,
            TerminalInputEvent::Paste {
                text: "clipboard".to_string(),
            }
        );
        assert_eq!(mobile.pending_paste(), Some("clipboard"));

        renderer.handle_viewport_change(390.0, 844.0, 3.0, Orientation::Portrait);
        mobile.handle_viewport_change(renderer.viewport());

        assert_eq!(mobile.pending_paste(), None);
        assert!(mobile.composition_buffer().is_empty());
        assert!(mobile.latched_modifiers().is_empty());
    }

    #[test]
    fn benchmark_summary_reports_nonzero_work() {
        let summary = benchmark_renderer(30, 24, 80, 100);
        assert_eq!(summary.frames, 30);
        assert_eq!(summary.updates_per_frame, 100);
        assert!(summary.p95_frame_time_ms >= 0.0);
    }

    #[test]
    fn default_web_palette_uses_dark_background_and_light_foreground() {
        let cell = TerminalCell {
            column: 0,
            text: "x".to_string(),
            foreground: TerminalColor::Default,
            background: TerminalColor::Default,
            bold: false,
            italic: false,
            underline: false,
            inverse: false,
        };

        let (foreground, background) = resolve_web_cell_colors(&cell);
        assert_eq!(foreground, DEFAULT_FOREGROUND_COLOR);
        assert_eq!(background, DEFAULT_BACKGROUND_COLOR);
    }

    #[test]
    fn inverse_web_palette_swaps_foreground_and_background() {
        let cell = TerminalCell {
            column: 0,
            text: "x".to_string(),
            foreground: TerminalColor::Indexed(2),
            background: TerminalColor::Indexed(4),
            bold: false,
            italic: false,
            underline: false,
            inverse: true,
        };

        let (foreground, background) = resolve_web_cell_colors(&cell);
        assert_eq!(foreground, indexed_web_color(4));
        assert_eq!(background, indexed_web_color(2));
    }
}
