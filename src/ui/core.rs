use crate::ui::selection::{SelectionState, TextLayout, TextPoint};

const MAX_UNDO_STACK: usize = 512;

#[derive(Clone)]
struct EditorSnapshot {
    text: String,
    cursor_byte: usize,
    selection: SelectionState,
    is_dirty: bool,
    preferred_column: Option<usize>,
}

#[derive(Clone, Default)]
pub struct EditorCore {
    pub text: String,
    pub cursor_byte: usize,
    pub selection: SelectionState,
    pub is_dirty: bool,
    preferred_column: Option<usize>,
    undo_stack: Vec<EditorSnapshot>,
    redo_stack: Vec<EditorSnapshot>,
}

impl EditorCore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor_byte = 0;
        self.selection.clear();
        self.is_dirty = false;
        self.preferred_column = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.cursor_byte = 0;
        self.selection.clear();
        self.is_dirty = false;
        self.preferred_column = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo_stack.pop() else {
            return false;
        };
        self.redo_stack.push(self.snapshot());
        self.restore(previous);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo_stack.pop() else {
            return false;
        };
        self.undo_stack.push(self.snapshot());
        self.restore(next);
        true
    }

    pub fn selection_byte_range(&self) -> Option<(usize, usize)> {
        let (start, end) = self.selection.normalized()?;
        let layout = self.layout();
        let start_byte = layout.point_to_byte(start);
        let end_byte = layout.point_to_byte(end);
        (start_byte < end_byte).then_some((start_byte, end_byte))
    }

    pub fn selected_text(&self) -> Option<String> {
        let (start, end) = self.selection_byte_range()?;
        Some(self.text[start..end].to_string())
    }

    pub fn delete_selection_if_any(&mut self) -> bool {
        let Some((start, end)) = self.selection_byte_range() else {
            return false;
        };
        self.text.replace_range(start..end, "");
        self.cursor_byte = start;
        self.selection.clear();
        self.preferred_column = None;
        self.is_dirty = true;
        true
    }

    pub fn insert_at_cursor(&mut self, text: &str) {
        self.push_undo_checkpoint();
        let _ = self.delete_selection_if_any();
        self.clamp_cursor_to_boundary();
        self.text.insert_str(self.cursor_byte, text);
        self.cursor_byte += text.len();
        self.preferred_column = None;
        self.is_dirty = true;
    }

    pub fn delete_backspace(&mut self) {
        self.push_undo_checkpoint();
        if self.delete_selection_if_any() {
            return;
        }
        self.clamp_cursor_to_boundary();
        if self.cursor_byte == 0 {
            return;
        }
        let prev = self.text[..self.cursor_byte]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.text.replace_range(prev..self.cursor_byte, "");
        self.cursor_byte = prev;
        self.preferred_column = None;
        self.is_dirty = true;
    }

    pub fn delete_forward(&mut self) {
        self.push_undo_checkpoint();
        if self.delete_selection_if_any() {
            return;
        }
        self.clamp_cursor_to_boundary();
        if self.cursor_byte >= self.text.len() {
            return;
        }
        let mut iter = self.text[self.cursor_byte..].char_indices();
        let _ = iter.next();
        let next = iter
            .next()
            .map(|(i, _)| self.cursor_byte + i)
            .unwrap_or(self.text.len());
        self.text.replace_range(self.cursor_byte..next, "");
        self.preferred_column = None;
        self.is_dirty = true;
    }

    pub fn move_left(&mut self, selecting: bool) {
        if !selecting && self.collapse_selection_to_start() {
            self.preferred_column = None;
            return;
        }

        self.clamp_cursor_to_boundary();
        let layout = self.layout();
        let before = layout.byte_to_point(self.cursor_byte);
        if self.cursor_byte == 0 {
            self.update_selection_for_motion(selecting, before, before, &layout);
            self.preferred_column = None;
            return;
        }
        self.cursor_byte = self.text[..self.cursor_byte]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
        let after = layout.byte_to_point(self.cursor_byte);
        self.update_selection_for_motion(selecting, before, after, &layout);
        self.preferred_column = None;
    }

    pub fn move_right(&mut self, selecting: bool) {
        if !selecting && self.collapse_selection_to_end() {
            self.preferred_column = None;
            return;
        }

        self.clamp_cursor_to_boundary();
        let layout = self.layout();
        let before = layout.byte_to_point(self.cursor_byte);
        if self.cursor_byte >= self.text.len() {
            self.update_selection_for_motion(selecting, before, before, &layout);
            self.preferred_column = None;
            return;
        }
        let mut iter = self.text[self.cursor_byte..].char_indices();
        let _ = iter.next();
        self.cursor_byte = iter
            .next()
            .map(|(i, _)| self.cursor_byte + i)
            .unwrap_or(self.text.len());
        let after = layout.byte_to_point(self.cursor_byte);
        self.update_selection_for_motion(selecting, before, after, &layout);
        self.preferred_column = None;
    }

    pub fn line_col_from_byte(&self, byte: usize) -> (usize, usize) {
        let layout = self.layout();
        let point = layout.byte_to_point(byte.min(self.text.len()));
        (point.line, point.column)
    }

    pub fn move_up(&mut self, selecting: bool) {
        if !selecting && self.collapse_selection_to_start() {
            let (_, col) = self.line_col_from_byte(self.cursor_byte);
            self.preferred_column = Some(col);
            return;
        }

        self.clamp_cursor_to_boundary();
        let layout = self.layout();
        let before = layout.byte_to_point(self.cursor_byte);
        let goal_col = self.preferred_column.unwrap_or(before.column);
        let line = before.line;
        if line == 0 {
            self.update_selection_for_motion(selecting, before, before, &layout);
            self.preferred_column = Some(goal_col);
            return;
        }
        let next_line = line - 1;
        let next_col = goal_col.min(layout.line_len(next_line));
        self.cursor_byte = layout.point_to_byte(TextPoint {
            line: next_line,
            column: next_col,
        });
        let after = layout.byte_to_point(self.cursor_byte);
        self.update_selection_for_motion(selecting, before, after, &layout);
        self.preferred_column = Some(goal_col);
    }

    pub fn move_down(&mut self, selecting: bool) {
        if !selecting && self.collapse_selection_to_end() {
            let (_, col) = self.line_col_from_byte(self.cursor_byte);
            self.preferred_column = Some(col);
            return;
        }

        self.clamp_cursor_to_boundary();
        let layout = self.layout();
        let before = layout.byte_to_point(self.cursor_byte);
        let goal_col = self.preferred_column.unwrap_or(before.column);
        let next_line = (before.line + 1).min(layout.line_count().saturating_sub(1));
        let next_col = goal_col.min(layout.line_len(next_line));
        self.cursor_byte = layout.point_to_byte(TextPoint {
            line: next_line,
            column: next_col,
        });
        let after = layout.byte_to_point(self.cursor_byte);
        self.update_selection_for_motion(selecting, before, after, &layout);
        self.preferred_column = Some(goal_col);
    }

    pub fn move_home(&mut self, selecting: bool) {
        if !selecting && self.collapse_selection_to_start() {
            self.preferred_column = None;
            return;
        }

        self.clamp_cursor_to_boundary();
        let layout = self.layout();
        let before_point = layout.byte_to_point(self.cursor_byte);
        let before_text = &self.text[..self.cursor_byte];
        self.cursor_byte = before_text.rfind('\n').map_or(0, |i| i + 1);
        let after = layout.byte_to_point(self.cursor_byte);
        self.update_selection_for_motion(selecting, before_point, after, &layout);
        self.preferred_column = None;
    }

    pub fn move_end(&mut self, selecting: bool) {
        if !selecting && self.collapse_selection_to_end() {
            self.preferred_column = None;
            return;
        }

        self.clamp_cursor_to_boundary();
        let layout = self.layout();
        let before = layout.byte_to_point(self.cursor_byte);
        let after = &self.text[self.cursor_byte..];
        self.cursor_byte = after
            .find('\n')
            .map(|i| self.cursor_byte + i)
            .unwrap_or(self.text.len());
        let after = layout.byte_to_point(self.cursor_byte);
        self.update_selection_for_motion(selecting, before, after, &layout);
        self.preferred_column = None;
    }

    pub fn layout(&self) -> TextLayout {
        TextLayout::from_text(&self.text)
    }

    pub fn mark_saved(&mut self) {
        self.is_dirty = false;
    }

    pub fn push_undo_checkpoint(&mut self) {
        if self.undo_stack.len() >= MAX_UNDO_STACK {
            let _ = self.undo_stack.remove(0);
        }
        self.undo_stack.push(self.snapshot());
        self.redo_stack.clear();
    }

    fn clamp_cursor_to_boundary(&mut self) {
        if self.cursor_byte > self.text.len() {
            self.cursor_byte = self.text.len();
        }
        while self.cursor_byte > 0 && !self.text.is_char_boundary(self.cursor_byte) {
            self.cursor_byte -= 1;
        }
    }

    fn snapshot(&self) -> EditorSnapshot {
        EditorSnapshot {
            text: self.text.clone(),
            cursor_byte: self.cursor_byte,
            selection: self.selection.clone(),
            is_dirty: self.is_dirty,
            preferred_column: self.preferred_column,
        }
    }

    fn restore(&mut self, snapshot: EditorSnapshot) {
        self.text = snapshot.text;
        self.cursor_byte = snapshot.cursor_byte.min(self.text.len());
        self.selection = snapshot.selection;
        self.is_dirty = snapshot.is_dirty;
        self.preferred_column = snapshot.preferred_column;
        self.clamp_cursor_to_boundary();
    }

    fn update_selection_for_motion(
        &mut self,
        selecting: bool,
        before: TextPoint,
        after: TextPoint,
        layout: &TextLayout,
    ) {
        if selecting {
            if self.selection.anchor.is_none() {
                self.selection.anchor = Some(before);
            }
            self.selection.head = Some(after);
        } else {
            self.selection.clear();
        }
        self.cursor_byte = layout.point_to_byte(after);
    }

    fn collapse_selection_to_start(&mut self) -> bool {
        let Some((start, _)) = self.selection_byte_range() else {
            return false;
        };
        self.cursor_byte = start;
        self.selection.clear();
        true
    }

    fn collapse_selection_to_end(&mut self) -> bool {
        let Some((_, end)) = self.selection_byte_range() else {
            return false;
        };
        self.cursor_byte = end;
        self.selection.clear();
        true
    }
}
