pub struct EditorViewModel {
    pub line_count: usize,
    pub max_line_cols: usize,
    pub visible_rows: usize,
    pub visible_cols: usize,
    pub scroll: f32,
    pub hscroll: f32,
    pub start_line: usize,
    pub start_col: usize,
    pub viewport_text: String,
    pub line_numbers: String,
    pub track_h: f32,
    pub htrack_w: f32,
}

pub fn compute_editor_view(
    text: &str,
    viewport_w: f32,
    viewport_h: f32,
    sidebar_width: f32,
    line_height: f32,
    glyph_width: f32,
    scroll: f32,
    hscroll: f32,
) -> EditorViewModel {
    let lines: Vec<&str> = text.split('\n').collect();
    let line_count = lines.len().max(1);
    let max_line_cols = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(1);

    let visible_rows = (((viewport_h - 180.0) / line_height).floor() as usize).clamp(12, 140);
    let visible_cols =
        (((viewport_w - sidebar_width - 170.0) / glyph_width).floor() as usize).clamp(24, 320);

    let max_scroll = line_count.saturating_sub(visible_rows) as f32;
    let clamped_scroll = scroll.clamp(0.0, max_scroll);
    let (start_line, end_line) = window(line_count, visible_rows, clamped_scroll);

    let max_hscroll = max_line_cols.saturating_sub(visible_cols) as f32;
    let mut clamped_hscroll = hscroll.clamp(0.0, max_hscroll);
    let (mut start_col, mut end_col) = window(max_line_cols.max(1), visible_cols, clamped_hscroll);

    let mut viewport_lines = lines[start_line..end_line]
        .iter()
        .map(|line| slice_line_by_cols(line, start_col, end_col))
        .collect::<Vec<_>>();

    // Safety fallback: when stale horizontal scroll shifts every visible line fully out of view,
    // reset to column 0 so text always remains visible.
    if start_col > 0 && viewport_lines.iter().all(|line| line.is_empty()) {
        clamped_hscroll = 0.0;
        (start_col, end_col) = window(max_line_cols.max(1), visible_cols, clamped_hscroll);
        viewport_lines = lines[start_line..end_line]
            .iter()
            .map(|line| slice_line_by_cols(line, start_col, end_col))
            .collect::<Vec<_>>();
    }
    let viewport_text = viewport_lines.join("\n");

    let line_number_width = line_count.to_string().len().max(2);
    let line_numbers = (start_line..end_line)
        .map(|line| format!("{:>width$}", line + 1, width = line_number_width))
        .collect::<Vec<_>>()
        .join("\n");

    let track_h = (visible_rows as f32 * 17.5).clamp(120.0, 820.0);
    let htrack_w = (visible_cols as f32 * 8.2).clamp(120.0, 1400.0);

    EditorViewModel {
        line_count,
        max_line_cols,
        visible_rows,
        visible_cols,
        scroll: clamped_scroll,
        hscroll: clamped_hscroll,
        start_line,
        start_col,
        viewport_text,
        line_numbers,
        track_h,
        htrack_w,
    }
}

fn window(total: usize, visible: usize, scroll: f32) -> (usize, usize) {
    let visible = visible.max(1);
    if total <= visible {
        return (0, total);
    }
    let max_start = total.saturating_sub(visible);
    let start = scroll.floor().clamp(0.0, max_start as f32) as usize;
    (start, (start + visible).min(total))
}

fn slice_line_by_cols(line: &str, start_col: usize, end_col: usize) -> String {
    line.chars()
        .skip(start_col)
        .take(end_col.saturating_sub(start_col))
        .collect()
}
