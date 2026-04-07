pub fn scrollbar_metrics(
    total: usize,
    visible: usize,
    start: usize,
    track_h: f32,
) -> (f32, f32, bool) {
    if total <= visible || total == 0 {
        return (track_h, 0.0, false);
    }

    let ratio = (visible as f32 / total as f32).clamp(0.08, 1.0);
    let thumb_h = (track_h * ratio).max(18.0).min(track_h);
    let max_start = total.saturating_sub(visible) as f32;
    let progress = (start as f32 / max_start).clamp(0.0, 1.0);
    let thumb_top = (track_h - thumb_h) * progress;
    (thumb_h, thumb_top, true)
}
