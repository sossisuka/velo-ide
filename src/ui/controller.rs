use crate::ui::menu::MenuCommand;
use gpui::KeyDownEvent;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppMenuAction {
    NewTextFile,
    OpenFile,
    OpenFolder,
    Save,
    Exit,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Find,
    Replace,
    SelectAll,
    ExpandSelection,
    CommandPalette,
    AppearancePanel,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ToggleTerminalPanel,
    ClearTerminal,
}

impl From<MenuCommand> for AppMenuAction {
    fn from(value: MenuCommand) -> Self {
        match value {
            MenuCommand::NewTextFile => Self::NewTextFile,
            MenuCommand::OpenFile => Self::OpenFile,
            MenuCommand::OpenFolder => Self::OpenFolder,
            MenuCommand::Save => Self::Save,
            MenuCommand::Exit => Self::Exit,
            MenuCommand::Undo => Self::Undo,
            MenuCommand::Redo => Self::Redo,
            MenuCommand::Cut => Self::Cut,
            MenuCommand::Copy => Self::Copy,
            MenuCommand::Paste => Self::Paste,
            MenuCommand::Find => Self::Find,
            MenuCommand::Replace => Self::Replace,
            MenuCommand::SelectAll => Self::SelectAll,
            MenuCommand::ExpandSelection => Self::ExpandSelection,
            MenuCommand::CommandPalette => Self::CommandPalette,
            MenuCommand::AppearancePanel => Self::AppearancePanel,
            MenuCommand::ZoomIn => Self::ZoomIn,
            MenuCommand::ZoomOut => Self::ZoomOut,
            MenuCommand::ZoomReset => Self::ZoomReset,
            MenuCommand::ToggleTerminalPanel => Self::ToggleTerminalPanel,
            MenuCommand::ClearTerminal => Self::ClearTerminal,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorKeyAction {
    Save,
    Undo,
    Redo,
    Copy,
    Cut,
    Paste,
    SelectAll,
    MoveLeft { selecting: bool },
    MoveRight { selecting: bool },
    MoveUp { selecting: bool },
    MoveDown { selecting: bool },
    MoveHome { selecting: bool },
    MoveEnd { selecting: bool },
    Backspace,
    Delete,
    Enter,
    Tab,
    InsertText(String),
    Ignore,
}

pub fn resolve_editor_key_action(event: &KeyDownEvent) -> EditorKeyAction {
    let mods = event.keystroke.modifiers;
    let cmd_or_ctrl = mods.control || mods.platform;
    let selecting = mods.shift;
    let key = event.keystroke.key.as_str();

    if cmd_or_ctrl && key.eq_ignore_ascii_case("s") {
        return EditorKeyAction::Save;
    }
    if cmd_or_ctrl && key.eq_ignore_ascii_case("z") {
        if selecting {
            return EditorKeyAction::Redo;
        }
        return EditorKeyAction::Undo;
    }
    if cmd_or_ctrl && key.eq_ignore_ascii_case("y") {
        return EditorKeyAction::Redo;
    }
    if cmd_or_ctrl && key.eq_ignore_ascii_case("c") {
        return EditorKeyAction::Copy;
    }
    if cmd_or_ctrl && key.eq_ignore_ascii_case("x") {
        return EditorKeyAction::Cut;
    }
    if cmd_or_ctrl && key.eq_ignore_ascii_case("v") {
        return EditorKeyAction::Paste;
    }
    if cmd_or_ctrl && key.eq_ignore_ascii_case("a") {
        return EditorKeyAction::SelectAll;
    }

    if cmd_or_ctrl || mods.alt || mods.function {
        return EditorKeyAction::Ignore;
    }

    match key {
        "left" => EditorKeyAction::MoveLeft { selecting },
        "right" => EditorKeyAction::MoveRight { selecting },
        "up" => EditorKeyAction::MoveUp { selecting },
        "down" => EditorKeyAction::MoveDown { selecting },
        "home" => EditorKeyAction::MoveHome { selecting },
        "end" => EditorKeyAction::MoveEnd { selecting },
        "backspace" => EditorKeyAction::Backspace,
        "delete" => EditorKeyAction::Delete,
        "enter" => EditorKeyAction::Enter,
        "tab" => EditorKeyAction::Tab,
        _ => event
            .keystroke
            .key_char
            .clone()
            .map(EditorKeyAction::InsertText)
            .unwrap_or(EditorKeyAction::Ignore),
    }
}
