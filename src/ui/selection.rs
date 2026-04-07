#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextPoint {
    pub line: usize,
    pub column: usize,
}

impl TextPoint {
    pub fn ordered(a: Self, b: Self) -> (Self, Self) {
        if (a.line, a.column) <= (b.line, b.column) {
            (a, b)
        } else {
            (b, a)
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TextMetrics {
    pub line_height: f32,
    pub glyph_width: f32,
    pub left_padding: f32,
    pub top_padding: f32,
    pub gutter_width: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ScrollOffset {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct ViewportCells {
    pub rows: usize,
    pub cols: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectionRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug, Default)]
pub struct SelectionState {
    pub anchor: Option<TextPoint>,
    pub head: Option<TextPoint>,
    pub dragging: bool,
}

impl SelectionState {
    pub fn clear(&mut self) {
        self.anchor = None;
        self.head = None;
        self.dragging = false;
    }

    pub fn begin_drag(&mut self, at: TextPoint) {
        self.anchor = Some(at);
        self.head = Some(at);
        self.dragging = true;
    }

    pub fn update_drag(&mut self, to: TextPoint) {
        if self.dragging {
            self.head = Some(to);
        }
    }

    pub fn end_drag(&mut self) {
        self.dragging = false;
    }

    pub fn normalized(&self) -> Option<(TextPoint, TextPoint)> {
        let (Some(a), Some(b)) = (self.anchor, self.head) else {
            return None;
        };
        if a == b {
            return None;
        }
        Some(TextPoint::ordered(a, b))
    }
}

#[derive(Clone, Debug, Default)]
pub struct TextLayout {
    line_starts: Vec<usize>,
    line_lengths: Vec<usize>,
}

impl TextLayout {
    pub fn from_text(text: &str) -> Self {
        let mut line_starts = vec![0usize];
        let mut line_lengths = Vec::new();
        let mut current_col = 0usize;

        for (i, ch) in text.char_indices() {
            if ch == '\n' {
                line_lengths.push(current_col);
                line_starts.push(i + ch.len_utf8());
                current_col = 0;
            } else {
                current_col += 1;
            }
        }

        line_lengths.push(current_col);

        Self {
            line_starts,
            line_lengths,
        }
    }

    pub fn line_count(&self) -> usize {
        self.line_lengths.len().max(1)
    }

    pub fn max_line_len(&self) -> usize {
        self.line_lengths.iter().copied().max().unwrap_or(0)
    }

    pub fn line_len(&self, line: usize) -> usize {
        let last = self.line_count().saturating_sub(1);
        self.line_lengths
            .get(line.min(last))
            .copied()
            .unwrap_or_default()
    }

    pub fn clamp_point(&self, point: TextPoint) -> TextPoint {
        let line = point.line.min(self.line_count().saturating_sub(1));
        let column = point.column.min(self.line_len(line));
        TextPoint { line, column }
    }

    pub fn point_to_byte(&self, point: TextPoint) -> usize {
        let p = self.clamp_point(point);
        let start = self.line_starts[p.line];
        start + p.column
    }

    pub fn byte_to_point(&self, byte: usize) -> TextPoint {
        if self.line_starts.is_empty() {
            return TextPoint::default();
        }

        let idx = self.line_starts.partition_point(|start| *start <= byte);
        let line = idx.saturating_sub(1).min(self.line_count().saturating_sub(1));
        let column = byte.saturating_sub(self.line_starts[line]).min(self.line_len(line));
        TextPoint { line, column }
    }

}

pub fn screen_to_text_point(
    x: f32,
    y: f32,
    metrics: TextMetrics,
    scroll: ScrollOffset,
    layout: &TextLayout,
) -> TextPoint {
    let row = ((y - metrics.top_padding) / metrics.line_height)
        .floor()
        .max(0.0) as usize;
    let visible_col = ((x - metrics.left_padding - metrics.gutter_width) / metrics.glyph_width)
        .floor()
        .max(0.0) as usize;

    layout.clamp_point(TextPoint {
        line: scroll.line + row,
        column: scroll.column + visible_col,
    })
}

pub fn hover_char_index(
    x: f32,
    y: f32,
    metrics: TextMetrics,
    scroll: ScrollOffset,
    layout: &TextLayout,
) -> usize {
    let point = screen_to_text_point(x, y, metrics, scroll, layout);
    layout.point_to_byte(point)
}

pub fn selection_rects(
    selection: &SelectionState,
    layout: &TextLayout,
    metrics: TextMetrics,
    scroll: ScrollOffset,
    viewport: ViewportCells,
) -> Vec<SelectionRect> {
    let Some((start, end)) = selection.normalized() else {
        return Vec::new();
    };

    let first_visible_line = scroll.line;
    let last_visible_line = scroll.line + viewport.rows.saturating_sub(1);
    let first_visible_col = scroll.column;
    let last_visible_col = scroll.column + viewport.cols;

    let mut rects = Vec::new();
    for line in start.line..=end.line {
        if line < first_visible_line || line > last_visible_line {
            continue;
        }

        let raw_start = if line == start.line { start.column } else { 0 };
        let raw_end = if line == end.line {
            end.column
        } else {
            layout.line_len(line)
        };

        let sel_start_col = raw_start.max(first_visible_col);
        let sel_end_col = raw_end.min(last_visible_col);
        if sel_end_col <= sel_start_col {
            continue;
        }

        let x = metrics.left_padding
            + metrics.gutter_width
            + ((sel_start_col - first_visible_col) as f32 * metrics.glyph_width);
        let y = metrics.top_padding + ((line - first_visible_line) as f32 * metrics.line_height);
        let width = (sel_end_col - sel_start_col) as f32 * metrics.glyph_width;

        rects.push(SelectionRect {
            x,
            y,
            width,
            height: metrics.line_height,
        });
    }

    rects
}
