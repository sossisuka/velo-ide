use crate::ui::core::EditorCore;
use gpui::{ClipboardItem, Context};

pub enum CopySelectionResult {
    Copied,
    NothingSelected,
}

pub enum CutSelectionResult {
    Cut,
    NothingSelected,
}

pub enum PasteResult {
    Pasted,
    ClipboardEmpty,
    ClipboardHasNoText,
}

pub fn copy_selection<T>(core: &EditorCore, cx: &mut Context<T>) -> CopySelectionResult {
    let Some(text) = core.selected_text() else {
        return CopySelectionResult::NothingSelected;
    };
    cx.write_to_clipboard(ClipboardItem::new_string(text));
    CopySelectionResult::Copied
}

pub fn cut_selection<T>(core: &mut EditorCore, cx: &mut Context<T>) -> CutSelectionResult {
    let Some(text) = core.selected_text() else {
        return CutSelectionResult::NothingSelected;
    };
    core.push_undo_checkpoint();
    cx.write_to_clipboard(ClipboardItem::new_string(text));
    let _ = core.delete_selection_if_any();
    CutSelectionResult::Cut
}

pub fn paste_from_clipboard<T>(core: &mut EditorCore, cx: &mut Context<T>) -> PasteResult {
    let Some(item) = cx.read_from_clipboard() else {
        return PasteResult::ClipboardEmpty;
    };
    let Some(text) = item.text() else {
        return PasteResult::ClipboardHasNoText;
    };
    core.insert_at_cursor(&text);
    PasteResult::Pasted
}
