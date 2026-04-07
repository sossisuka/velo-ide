pub struct ExplorerViewModel {
    pub visible_rows: usize,
    pub scroll: f32,
    pub start: usize,
    pub end: usize,
    pub track_h: f32,
}

pub fn compute_explorer_view(
    entry_count: usize,
    viewport_h: f32,
    scroll: f32,
) -> ExplorerViewModel {
    let visible_rows = (((viewport_h - 250.0) / 22.0).floor() as usize).clamp(8, 80);
    let max_scroll = entry_count.saturating_sub(visible_rows) as f32;
    let clamped_scroll = scroll.clamp(0.0, max_scroll);
    let (start, end) = window(entry_count, visible_rows, clamped_scroll);
    let track_h = (visible_rows as f32 * 22.0).clamp(150.0, 760.0);

    ExplorerViewModel {
        visible_rows,
        scroll: clamped_scroll,
        start,
        end,
        track_h,
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
