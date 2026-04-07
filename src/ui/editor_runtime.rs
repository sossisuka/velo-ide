use crate::ui::{controller::EditorKeyAction, core::EditorCore};

pub enum CoreKeyApply {
    Applied,
    NotHandled,
}

pub fn apply_core_key_action(core: &mut EditorCore, action: &EditorKeyAction) -> CoreKeyApply {
    match action {
        EditorKeyAction::MoveLeft { selecting } => core.move_left(*selecting),
        EditorKeyAction::MoveRight { selecting } => core.move_right(*selecting),
        EditorKeyAction::MoveUp { selecting } => core.move_up(*selecting),
        EditorKeyAction::MoveDown { selecting } => core.move_down(*selecting),
        EditorKeyAction::MoveHome { selecting } => core.move_home(*selecting),
        EditorKeyAction::MoveEnd { selecting } => core.move_end(*selecting),
        EditorKeyAction::Backspace => core.delete_backspace(),
        EditorKeyAction::Delete => core.delete_forward(),
        EditorKeyAction::Enter => core.insert_at_cursor("\n"),
        EditorKeyAction::Tab => core.insert_at_cursor("    "),
        EditorKeyAction::InsertText(text) => core.insert_at_cursor(text),
        _ => return CoreKeyApply::NotHandled,
    }
    CoreKeyApply::Applied
}
