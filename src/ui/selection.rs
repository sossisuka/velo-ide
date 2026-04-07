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
    line_column_bytes: Vec<Vec<usize>>,
}

impl TextLayout {
    pub fn from_text(text: &str) -> Self {
        let mut line_starts = vec![0usize];
        let mut line_lengths = Vec::new();
        let mut line_column_bytes = vec![vec![0usize]];

        for (i, ch) in text.char_indices() {
            if ch == '\n' {
                let current_len = line_column_bytes
                    .last()
                    .map_or(0, |line| line.len().saturating_sub(1));
                line_lengths.push(current_len);
                let next_start = i + ch.len_utf8();
                line_starts.push(next_start);
                line_column_bytes.push(vec![next_start]);
            } else if let Some(current_line) = line_column_bytes.last_mut() {
                current_line.push(i + ch.len_utf8());
            }
        }

        let current_len = line_column_bytes
            .last()
            .map_or(0, |line| line.len().saturating_sub(1));
        line_lengths.push(current_len);

        Self {
            line_starts,
            line_lengths,
            line_column_bytes,
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
        self.line_column_bytes
            .get(p.line)
            .and_then(|line| line.get(p.column))
            .copied()
            .unwrap_or_else(|| self.line_starts[p.line])
    }

    pub fn byte_to_point(&self, byte: usize) -> TextPoint {
        if self.line_starts.is_empty() {
            return TextPoint::default();
        }

        let idx = self.line_starts.partition_point(|start| *start <= byte);
        let line = idx
            .saturating_sub(1)
            .min(self.line_count().saturating_sub(1));
        let line_cols = &self.line_column_bytes[line];
        let line_start = *line_cols.first().unwrap_or(&0);
        let line_end = *line_cols.last().unwrap_or(&line_start);
        let clamped = byte.clamp(line_start, line_end);
        let column = line_cols
            .partition_point(|b| *b <= clamped)
            .saturating_sub(1)
            .min(self.line_len(line));
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

pub fn selection_byte_ranges_in_viewport(
    selection: &SelectionState,
    layout: &TextLayout,
    scroll: ScrollOffset,
    viewport: ViewportCells,
    viewport_text: &str,
) -> Vec<Range<usize>> {
    let Some((start, end)) = selection.normalized() else {
        return Vec::new();
    };

    let first_visible_line = scroll.line;
    let last_visible_line = scroll.line + viewport.rows.saturating_sub(1);
    let first_visible_col = scroll.column;
    let last_visible_col = scroll.column + viewport.cols;

    let viewport_layout = TextLayout::from_text(viewport_text);
    let viewport_lines = viewport_layout.line_count();

    let mut ranges = Vec::new();
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

        let local_line = line - first_visible_line;
        if local_line >= viewport_lines {
            continue;
        }

        let local_start_col = sel_start_col.saturating_sub(first_visible_col);
        let local_end_col = sel_end_col.saturating_sub(first_visible_col);

        let start_byte = viewport_layout.point_to_byte(TextPoint {
            line: local_line,
            column: local_start_col,
        });
        let end_byte = viewport_layout.point_to_byte(TextPoint {
            line: local_line,
            column: local_end_col,
        });

        if start_byte < end_byte {
            ranges.push(start_byte..end_byte);
        }
    }

    ranges
}
use std::ops::Range;
