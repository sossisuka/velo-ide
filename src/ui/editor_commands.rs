use crate::ui::core::EditorCore;
use crate::ui::selection::TextPoint;

pub fn select_all(core: &mut EditorCore) {
    let layout = core.layout();
    let last_line = layout.line_count().saturating_sub(1);
    let last_col = layout.line_len(last_line);
    core.selection.anchor = Some(TextPoint { line: 0, column: 0 });
    core.selection.head = Some(TextPoint {
        line: last_line,
        column: last_col,
    });
    core.cursor_byte = core.text.len();
}

pub fn expand_selection(core: &mut EditorCore) {
    let layout = core.layout();
    let current = layout.byte_to_point(core.cursor_byte);
    let next = TextPoint {
        line: current.line,
        column: (current.column + 1).min(layout.line_len(current.line)),
    };
    core.selection.anchor = Some(current);
    core.selection.head = Some(next);
    core.cursor_byte = layout.point_to_byte(next);
}

pub fn cursor_status(core: &EditorCore) -> String {
    let (line, col) = core.line_col_from_byte(core.cursor_byte);
    let lines = core.layout().line_count();
    if core.is_dirty {
        format!(
            "Modified (Ctrl+S to save) | Ln {}, Col {} | {} lines",
            line + 1,
            col + 1,
            lines
        )
    } else {
        format!("Ln {}, Col {} | {} lines", line + 1, col + 1, lines)
    }
}
