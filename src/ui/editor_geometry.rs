use crate::ui::selection::{self, ScrollOffset, TextLayout, TextMetrics, ViewportCells};

pub fn viewport_cells(
    viewport_w: f32,
    viewport_h: f32,
    sidebar_width: f32,
    line_height: f32,
    glyph_width: f32,
) -> ViewportCells {
    let rows = (((viewport_h - 180.0) / line_height).floor() as usize).clamp(12, 140);
    let cols =
        (((viewport_w - sidebar_width - 170.0) / glyph_width).floor() as usize).clamp(24, 320);
    ViewportCells { rows, cols }
}

pub fn scroll_offset(
    editor_scroll: f32,
    editor_hscroll: f32,
    layout: &TextLayout,
    viewport: ViewportCells,
) -> ScrollOffset {
    let max_line_start = layout.line_count().saturating_sub(viewport.rows.max(1));
    let max_col_start = layout.max_line_len().saturating_sub(viewport.cols.max(1));

    ScrollOffset {
        line: editor_scroll.floor().clamp(0.0, max_line_start as f32) as usize,
        column: editor_hscroll.floor().clamp(0.0, max_col_start as f32) as usize,
    }
}

pub fn hit_test_byte(
    x: f32,
    y: f32,
    metrics: TextMetrics,
    scroll: ScrollOffset,
    layout: &TextLayout,
) -> usize {
    selection::hover_char_index(x, y, metrics, scroll, layout)
}
