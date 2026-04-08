#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TopMenuId {
    File,
    Edit,
    Selection,
    View,
    Terminal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MenuCommand {
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

#[derive(Clone, Copy, Debug)]
pub struct MenuItem {
    pub id: &'static str,
    pub label: &'static str,
    pub command: Option<MenuCommand>,
    pub keybinding: Option<&'static str>,
    pub submenu: &'static [MenuItem],
}

#[derive(Clone, Copy, Debug)]
pub struct TopMenu {
    pub id: TopMenuId,
    pub label: &'static str,
    pub items: &'static [MenuItem],
}

const FILE_ITEMS: &[MenuItem] = &[
    MenuItem {
        id: "file.newTextFile",
        label: "New Text File",
        command: Some(MenuCommand::NewTextFile),
        keybinding: Some("Ctrl+N"),
        submenu: &[],
    },
    MenuItem {
        id: "file.openFile",
        label: "Open File...",
        command: Some(MenuCommand::OpenFile),
        keybinding: Some("Ctrl+O"),
        submenu: &[],
    },
    MenuItem {
        id: "file.openFolder",
        label: "Open Folder...",
        command: Some(MenuCommand::OpenFolder),
        keybinding: Some("Ctrl+K Ctrl+O"),
        submenu: &[],
    },
    MenuItem {
        id: "file.save",
        label: "Save",
        command: Some(MenuCommand::Save),
        keybinding: Some("Ctrl+S"),
        submenu: &[],
    },
    MenuItem {
        id: "file.exit",
        label: "Exit",
        command: Some(MenuCommand::Exit),
        keybinding: Some("Alt+F4"),
        submenu: &[],
    },
];

const EDIT_ITEMS: &[MenuItem] = &[
    MenuItem {
        id: "edit.undo",
        label: "Undo",
        command: Some(MenuCommand::Undo),
        keybinding: Some("Ctrl+Z"),
        submenu: &[],
    },
    MenuItem {
        id: "edit.redo",
        label: "Redo",
        command: Some(MenuCommand::Redo),
        keybinding: Some("Ctrl+Y"),
        submenu: &[],
    },
    MenuItem {
        id: "edit.cut",
        label: "Cut",
        command: Some(MenuCommand::Cut),
        keybinding: Some("Ctrl+X"),
        submenu: &[],
    },
    MenuItem {
        id: "edit.copy",
        label: "Copy",
        command: Some(MenuCommand::Copy),
        keybinding: Some("Ctrl+C"),
        submenu: &[],
    },
    MenuItem {
        id: "edit.paste",
        label: "Paste",
        command: Some(MenuCommand::Paste),
        keybinding: Some("Ctrl+V"),
        submenu: &[],
    },
    MenuItem {
        id: "edit.find",
        label: "Find",
        command: Some(MenuCommand::Find),
        keybinding: Some("Ctrl+F"),
        submenu: &[],
    },
    MenuItem {
        id: "edit.replace",
        label: "Replace",
        command: Some(MenuCommand::Replace),
        keybinding: Some("Ctrl+H"),
        submenu: &[],
    },
];

const SELECTION_ITEMS: &[MenuItem] = &[
    MenuItem {
        id: "selection.selectAll",
        label: "Select All",
        command: Some(MenuCommand::SelectAll),
        keybinding: Some("Ctrl+A"),
        submenu: &[],
    },
    MenuItem {
        id: "selection.expandSelection",
        label: "Expand Selection",
        command: Some(MenuCommand::ExpandSelection),
        keybinding: Some("Shift+Alt+Right"),
        submenu: &[],
    },
];

const APPEARANCE_SUBMENU: &[MenuItem] = &[
    MenuItem {
        id: "view.zoomIn",
        label: "Zoom In",
        command: Some(MenuCommand::ZoomIn),
        keybinding: Some("Ctrl++"),
        submenu: &[],
    },
    MenuItem {
        id: "view.zoomOut",
        label: "Zoom Out",
        command: Some(MenuCommand::ZoomOut),
        keybinding: Some("Ctrl+-"),
        submenu: &[],
    },
    MenuItem {
        id: "view.zoomReset",
        label: "Reset Zoom",
        command: Some(MenuCommand::ZoomReset),
        keybinding: Some("Ctrl+0"),
        submenu: &[],
    },
];

const VIEW_ITEMS: &[MenuItem] = &[
    MenuItem {
        id: "view.commandPalette",
        label: "Command Palette...",
        command: Some(MenuCommand::CommandPalette),
        keybinding: Some("Ctrl+Shift+P"),
        submenu: &[],
    },
    MenuItem {
        id: "view.appearance",
        label: "Appearance",
        command: Some(MenuCommand::AppearancePanel),
        keybinding: None,
        submenu: APPEARANCE_SUBMENU,
    },
    MenuItem {
        id: "view.zoomIn",
        label: "Zoom In",
        command: Some(MenuCommand::ZoomIn),
        keybinding: Some("Ctrl++"),
        submenu: &[],
    },
];

const TERMINAL_ITEMS: &[MenuItem] = &[
    MenuItem {
        id: "terminal.toggle",
        label: "Toggle Panel",
        command: Some(MenuCommand::ToggleTerminalPanel),
        keybinding: Some("Ctrl+J"),
        submenu: &[],
    },
    MenuItem {
        id: "terminal.clear",
        label: "Clear Terminal",
        command: Some(MenuCommand::ClearTerminal),
        keybinding: None,
        submenu: &[],
    },
];

pub const TOP_MENUS: &[TopMenu] = &[
    TopMenu {
        id: TopMenuId::File,
        label: "File",
        items: FILE_ITEMS,
    },
    TopMenu {
        id: TopMenuId::Edit,
        label: "Edit",
        items: EDIT_ITEMS,
    },
    TopMenu {
        id: TopMenuId::Selection,
        label: "Selection",
        items: SELECTION_ITEMS,
    },
    TopMenu {
        id: TopMenuId::View,
        label: "View",
        items: VIEW_ITEMS,
    },
    TopMenu {
        id: TopMenuId::Terminal,
        label: "Terminal",
        items: TERMINAL_ITEMS,
    },
];
