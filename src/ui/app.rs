use crate::ui::{
    controller::{resolve_editor_key_action, AppMenuAction, EditorKeyAction},
    core::EditorCore,
    editor_commands, editor_geometry,
    editor_io::{self, CopySelectionResult, CutSelectionResult, PasteResult},
    editor_runtime::{self, CoreKeyApply},
    editor_view::compute_editor_view,
    explorer_view::compute_explorer_view,
    file_text::decode_text_file,
    menu::{MenuCommand, MenuItem, TopMenu, TopMenuId, TOP_MENUS},
    scroll::scrollbar_metrics,
    selection::{self, ScrollOffset, TextLayout, TextMetrics, ViewportCells},
    workspace::{FileEntry, VisibleKind, WorkspaceState},
    workspace_io::{self, OpenFileResult, SaveResult},
};
use std::{
    fs,
    path::{Path, PathBuf},
};

use gpui::{
    div, font, img, px, rgb, AnyElement, ClickEvent, Context, FocusHandle, Font, FontFallbacks,
    HighlightStyle,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement, Render, ScrollDelta, ScrollWheelEvent, SharedString,
    StatefulInteractiveElement, Styled, StyledText, Window,
};

#[derive(Clone)]
pub struct Icons {
    file_icons_base: PathBuf,
    activity_icons_base: PathBuf,
}

impl Icons {
    pub fn from_dirs(file_icons_dir: &Path, activity_icons_dir: &Path) -> Self {
        Self {
            file_icons_base: file_icons_dir.to_path_buf(),
            activity_icons_base: activity_icons_dir.to_path_buf(),
        }
    }

    fn by_name(&self, name: &str) -> PathBuf {
        self.file_icons_base.join(format!("{name}.svg"))
    }

    fn activity_png(&self, name: &str) -> PathBuf {
        self.activity_icons_base.join(format!("{name}.png"))
    }

    fn back(&self) -> PathBuf {
        self.activity_png("back")
    }

    fn next(&self) -> PathBuf {
        self.activity_png("next")
    }

    fn close_tab(&self) -> PathBuf {
        self.activity_png("close")
    }

    fn add(&self) -> PathBuf {
        self.activity_png("add")
    }

    fn folder(&self) -> PathBuf {
        self.activity_png("folder")
    }

    fn folder_open(&self) -> PathBuf {
        self.activity_png("folder")
    }

    fn settings(&self) -> PathBuf {
        self.activity_png("settings")
    }

    fn activity_explorer(&self) -> PathBuf {
        self.activity_png("folder")
    }

    fn activity_search(&self) -> PathBuf {
        self.activity_png("menu")
    }

    fn activity_source_control(&self) -> PathBuf {
        self.activity_png("refresh")
    }

    fn activity_run(&self) -> PathBuf {
        self.activity_png("play")
    }

    fn activity_extensions(&self) -> PathBuf {
        self.activity_png("plus")
    }

    fn terminal(&self) -> PathBuf {
        self.activity_png("menu")
    }

    fn terminal_clear(&self) -> PathBuf {
        self.activity_png("trash")
    }

    fn terminal_run(&self) -> PathBuf {
        self.activity_png("play")
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Welcome,
    Editor,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ActivityPanel {
    Explorer,
    Search,
    SourceControl,
    Run,
    Extensions,
    Settings,
}

const EDITOR_LINE_HEIGHT: f32 = 18.0;
const EDITOR_GLYPH_WIDTH: f32 = 8.2;
const EDITOR_LEFT_PADDING: f32 = 0.0;
const EDITOR_TOP_PADDING: f32 = 0.0;
const EDITOR_GUTTER_WIDTH: f32 = 0.0;
const MENU_BAR_HEIGHT: f32 = 34.0;
const MENU_BUTTON_WIDTH: f32 = 78.0;
const MENU_ITEM_HEIGHT: f32 = 28.0;
const MENU_PANEL_WIDTH: f32 = 300.0;
const RECENT_PROJECTS_LIMIT: usize = 8;

pub struct VeloIde {
    icons: Icons,
    screen: Screen,
    editor_focus: FocusHandle,

    workspace: WorkspaceState,
    sidebar_width: f32,
    resizing_sidebar: bool,
    resize_start_x: f32,
    resize_start_width: f32,
    explorer_scroll: f32,
    editor_scroll: f32,
    editor_hscroll: f32,
    tab_scroll: usize,
    last_viewport_width: f32,
    active_panel: ActivityPanel,

    core: EditorCore,
    hover_byte: Option<usize>,
    open_menu: Option<TopMenuId>,
    open_submenu: Option<&'static str>,
    status: SharedString,
    recent_projects: Vec<PathBuf>,
    terminal_open: bool,
    terminal_height: f32,
    terminal_lines: Vec<SharedString>,
}

impl VeloIde {
    fn compact_label(path_like: &str, max_chars: usize) -> SharedString {
        let file = Path::new(path_like)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| path_like.to_string());
        let mut chars = file.chars().collect::<Vec<_>>();
        if chars.len() > max_chars {
            chars.truncate(max_chars.saturating_sub(1));
            chars.push('.');
        }
        chars.into_iter().collect::<String>().into()
    }

    pub fn new(icons: Icons, cx: &mut Context<Self>) -> Self {
        let recent_projects = Self::load_recent_projects();
        Self {
            icons,
            screen: Screen::Welcome,
            editor_focus: cx.focus_handle(),
            workspace: WorkspaceState::default(),
            sidebar_width: 300.0,
            resizing_sidebar: false,
            resize_start_x: 0.0,
            resize_start_width: 300.0,
            explorer_scroll: 0.0,
            editor_scroll: 0.0,
            editor_hscroll: 0.0,
            tab_scroll: 0,
            last_viewport_width: 1360.0,
            active_panel: ActivityPanel::Explorer,
            core: EditorCore::new(),
            hover_byte: None,
            open_menu: None,
            open_submenu: None,
            status: "Ready".into(),
            recent_projects,
            terminal_open: false,
            terminal_height: 210.0,
            terminal_lines: vec![
                "Velo Terminal ready".into(),
                "PS D:\\Projects\\VeloCode>".into(),
            ],
        }
    }

    fn base_ui_font() -> Font {
        let mut f = font("Poppins");
        f.fallbacks = Some(FontFallbacks::from_fonts(vec!["Montserrat".to_string()]));
        f
    }

    fn cyrillic_ui_font() -> Font {
        let mut f = font("Montserrat");
        f.fallbacks = Some(FontFallbacks::from_fonts(vec!["Poppins".to_string()]));
        f
    }

    fn has_cyrillic(text: &str) -> bool {
        text.chars().any(|c| {
            ('\u{0400}'..='\u{04FF}').contains(&c) || ('\u{0500}'..='\u{052F}').contains(&c)
        })
    }

    fn localized_ui_font(text: &str) -> Font {
        if Self::has_cyrillic(text) {
            Self::cyrillic_ui_font()
        } else {
            Self::base_ui_font()
        }
    }

    fn recent_projects_file() -> PathBuf {
        if let Ok(app_data) = std::env::var("APPDATA") {
            let dir = PathBuf::from(app_data).join("Velo");
            let _ = fs::create_dir_all(&dir);
            return dir.join("recent_projects.txt");
        }

        if let Ok(home) = std::env::var("HOME") {
            let dir = PathBuf::from(home).join(".velo");
            let _ = fs::create_dir_all(&dir);
            return dir.join("recent_projects.txt");
        }

        PathBuf::from("recent_projects.txt")
    }

    fn normalize_project_key(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/").to_lowercase()
    }

    fn load_recent_projects() -> Vec<PathBuf> {
        let path = Self::recent_projects_file();
        let Ok(raw) = fs::read_to_string(path) else {
            return Vec::new();
        };

        let mut out = Vec::new();
        for line in raw.lines() {
            let item = line.trim();
            if item.is_empty() {
                continue;
            }
            let candidate = PathBuf::from(item);
            if candidate.is_dir() {
                out.push(candidate);
            }
            if out.len() >= RECENT_PROJECTS_LIMIT {
                break;
            }
        }
        out
    }

    fn persist_recent_projects(&self) {
        let path = Self::recent_projects_file();
        let body = self
            .recent_projects
            .iter()
            .take(RECENT_PROJECTS_LIMIT)
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let _ = fs::write(path, body);
    }

    fn remember_recent_project(&mut self, root: &Path) {
        let key = Self::normalize_project_key(root);
        self.recent_projects
            .retain(|p| Self::normalize_project_key(p) != key);
        self.recent_projects.insert(0, root.to_path_buf());
        if self.recent_projects.len() > RECENT_PROJECTS_LIMIT {
            self.recent_projects.truncate(RECENT_PROJECTS_LIMIT);
        }
        self.persist_recent_projects();
    }

    fn open_project_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(folder) = rfd::FileDialog::new()
            .set_title("Open Project Folder")
            .pick_folder()
        {
            self.load_project(folder, window, cx);
        }
    }

    fn load_project(&mut self, root: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.remember_recent_project(&root);
        self.workspace.load_project_index(root.clone(), 4000);
        self.core.clear();
        self.hover_byte = None;
        self.editor_scroll = 0.0;
        self.editor_hscroll = 0.0;
        self.tab_scroll = 0;
        self.screen = Screen::Editor;
        self.status = format!(
            "Opened project: {} ({} files)",
            root.display(),
            self.workspace.files.len()
        )
        .into();

        if !self.workspace.files.is_empty() {
            self.open_file_at(0, window, cx);
        }

        cx.notify();
    }

    fn open_file_at(&mut self, idx: usize, window: &mut Window, cx: &mut Context<Self>) {
        match workspace_io::open_file_into_editor(&mut self.workspace, &mut self.core, idx) {
            OpenFileResult::InvalidIndex => {}
            OpenFileResult::OpenFailed(err) => {
                self.status = format!("Open failed: {}", err).into();
            }
            OpenFileResult::Opened { status } => {
                self.hover_byte = None;
                self.editor_scroll = 0.0;
                self.editor_hscroll = 0.0;
                self.tab_scroll = self.workspace.open_tabs.len().saturating_sub(6);
                self.status = status.into();
                window.focus(&self.editor_focus);
            }
        }

        cx.notify();
    }

    fn close_file_tab(&mut self, idx: usize, cx: &mut Context<Self>) {
        let Some(pos) = self.workspace.open_tabs.iter().position(|tab| *tab == idx) else {
            return;
        };
        self.workspace.open_tabs.remove(pos);

        if self.workspace.active_index == Some(idx) {
            if self.workspace.open_tabs.is_empty() {
                self.workspace.active_index = None;
                self.core.clear();
                self.status = "No file open".into();
            } else {
                let next_pos = pos.min(self.workspace.open_tabs.len().saturating_sub(1));
                let next_idx = self.workspace.open_tabs[next_pos];
                self.workspace.active_index = Some(next_idx);
                if let Some(file) = self.workspace.files.get(next_idx) {
                    if let Ok(raw) = fs::read_to_string(&file.abs_path) {
                        let normalized = raw.replace("\r\n", "\n").replace('\r', "\n");
                        self.core.set_text(normalized);
                        self.status = format!("Opened {}", file.rel_path).into();
                    } else if let Ok(decoded) = decode_text_file(&file.abs_path) {
                        let normalized = decoded.text.replace("\r\n", "\n").replace('\r', "\n");
                        self.core.set_text(normalized);
                        self.status = format!("Opened {}", file.rel_path).into();
                    }
                }
            }
        } else {
            self.status = "Tab closed".into();
        }

        self.tab_scroll = self.tab_scroll.min(self.workspace.open_tabs.len().saturating_sub(1));
        cx.notify();
    }

    fn save_active_file(&mut self, cx: &mut Context<Self>) {
        match workspace_io::save_active_file(&self.workspace, &mut self.core) {
            SaveResult::NoFileSelected => {
                self.status = "No file selected".into();
            }
            SaveResult::SaveFailed(err) => {
                self.status = format!("Save failed: {}", err).into();
            }
            SaveResult::Saved { status } => {
                self.status = status.into();
            }
        }

        cx.notify();
    }

    fn clamp_sidebar_width(&mut self) {
        let min_sidebar = 220.0;
        let max_sidebar = (self.last_viewport_width - 420.0).max(min_sidebar);
        self.sidebar_width = self.sidebar_width.clamp(min_sidebar, max_sidebar);
    }

    fn start_sidebar_resize(&mut self, event: &MouseDownEvent) {
        if event.button != MouseButton::Left {
            return;
        }
        self.resizing_sidebar = true;
        self.resize_start_x = f32::from(event.position.x);
        self.resize_start_width = self.sidebar_width;
    }

    fn drag_sidebar_resize(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if !self.resizing_sidebar {
            return;
        }
        let dx = f32::from(event.position.x) - self.resize_start_x;
        self.sidebar_width = self.resize_start_width + dx;
        self.clamp_sidebar_width();
        cx.notify();
    }

    fn stop_sidebar_resize(&mut self, event: &MouseUpEvent, cx: &mut Context<Self>) {
        if event.button != MouseButton::Left || !self.resizing_sidebar {
            return;
        }
        self.resizing_sidebar = false;
        cx.notify();
    }

    fn reset_sidebar_width(&mut self, cx: &mut Context<Self>) {
        self.sidebar_width = 300.0;
        self.clamp_sidebar_width();
        cx.notify();
    }

    fn set_active_panel(&mut self, panel: ActivityPanel, cx: &mut Context<Self>) {
        self.active_panel = panel;
        cx.notify();
    }

    fn scroll_explorer(&mut self, event: &ScrollWheelEvent, cx: &mut Context<Self>) {
        let lines_delta = match event.delta {
            ScrollDelta::Lines(p) => p.y,
            ScrollDelta::Pixels(p) => f32::from(p.y) / 20.0,
        };
        self.explorer_scroll -= lines_delta;
        if self.explorer_scroll < 0.0 {
            self.explorer_scroll = 0.0;
        }
        cx.notify();
    }

    fn scroll_editor(&mut self, event: &ScrollWheelEvent, cx: &mut Context<Self>) {
        let (dx, dy) = match event.delta {
            ScrollDelta::Lines(p) => (p.x, p.y),
            ScrollDelta::Pixels(p) => (f32::from(p.x) / 20.0, f32::from(p.y) / 20.0),
        };

        // Shift+wheel or dominant horizontal delta scrolls code view horizontally.
        if event.modifiers.shift || dx.abs() > dy.abs() {
            self.editor_hscroll -= if dx.abs() > f32::EPSILON { dx } else { dy };
            if self.editor_hscroll < 0.0 {
                self.editor_hscroll = 0.0;
            }
        } else {
            self.editor_scroll -= dy;
            if self.editor_scroll < 0.0 {
                self.editor_scroll = 0.0;
            }
        }
        cx.notify();
    }

    fn editor_text_metrics(&self) -> TextMetrics {
        TextMetrics {
            line_height: EDITOR_LINE_HEIGHT,
            glyph_width: EDITOR_GLYPH_WIDTH,
            left_padding: EDITOR_LEFT_PADDING,
            top_padding: EDITOR_TOP_PADDING,
            gutter_width: EDITOR_GUTTER_WIDTH,
        }
    }

    fn hit_test_editor(&self, x: f32, y: f32, window: &Window) -> (TextLayout, usize) {
        let layout = self.core.layout();
        let viewport = editor_geometry::viewport_cells(
            f32::from(window.viewport_size().width),
            f32::from(window.viewport_size().height),
            self.sidebar_width,
            EDITOR_LINE_HEIGHT,
            EDITOR_GLYPH_WIDTH,
        );
        let scroll = editor_geometry::scroll_offset(
            self.editor_scroll,
            self.editor_hscroll,
            &layout,
            viewport,
        );
        let byte =
            editor_geometry::hit_test_byte(x, y, self.editor_text_metrics(), scroll, &layout);
        (layout, byte)
    }

    fn begin_selection_drag(
        &mut self,
        event: &MouseDownEvent,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if self.workspace.active_index.is_none() || event.button != MouseButton::Left {
            return;
        }
        let x = f32::from(event.position.x);
        let y = f32::from(event.position.y);
        let (layout, byte) = self.hit_test_editor(x, y, window);
        let point = layout.byte_to_point(byte);
        self.core.cursor_byte = byte;
        self.core.selection.begin_drag(point);
        self.hover_byte = Some(byte);
        self.refresh_status();
        cx.notify();
    }

    fn update_selection_drag(
        &mut self,
        event: &MouseMoveEvent,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if self.workspace.active_index.is_none() {
            return;
        }
        let x = f32::from(event.position.x);
        let y = f32::from(event.position.y);
        let (layout, byte) = self.hit_test_editor(x, y, window);

        if self.hover_byte != Some(byte) {
            self.hover_byte = Some(byte);
        }

        if self.core.selection.dragging && event.dragging() {
            self.core.cursor_byte = byte;
            self.core.selection.update_drag(layout.byte_to_point(byte));
            self.refresh_status();
            cx.notify();
        }
    }

    fn end_selection_drag(
        &mut self,
        event: &MouseUpEvent,
        _window: &Window,
        cx: &mut Context<Self>,
    ) {
        if event.button != MouseButton::Left || !self.core.selection.dragging {
            return;
        }
        self.core.selection.end_drag();
        self.refresh_status();
        cx.notify();
    }

    fn undo_edit(&mut self, cx: &mut Context<Self>) {
        if !self.core.undo() {
            self.status = "Nothing to undo".into();
            cx.notify();
            return;
        }
        self.hover_byte = None;
        self.refresh_status();
        cx.notify();
    }

    fn redo_edit(&mut self, cx: &mut Context<Self>) {
        if !self.core.redo() {
            self.status = "Nothing to redo".into();
            cx.notify();
            return;
        }
        self.hover_byte = None;
        self.refresh_status();
        cx.notify();
    }

    fn copy_selection(&mut self, cx: &mut Context<Self>) -> bool {
        match editor_io::copy_selection(&self.core, cx) {
            CopySelectionResult::Copied => {
                self.status = "Selection copied".into();
                true
            }
            CopySelectionResult::NothingSelected => {
                self.status = "Nothing selected".into();
                false
            }
        }
    }

    fn cut_selection(&mut self, cx: &mut Context<Self>) -> bool {
        match editor_io::cut_selection(&mut self.core, cx) {
            CutSelectionResult::Cut => {
                self.hover_byte = None;
                self.status = "Selection cut".into();
                true
            }
            CutSelectionResult::NothingSelected => {
                self.status = "Nothing selected".into();
                false
            }
        }
    }

    fn paste_from_clipboard(&mut self, cx: &mut Context<Self>) -> bool {
        match editor_io::paste_from_clipboard(&mut self.core, cx) {
            PasteResult::Pasted => {
                self.hover_byte = None;
                self.status = "Pasted from clipboard".into();
                true
            }
            PasteResult::ClipboardEmpty => {
                self.status = "Clipboard is empty".into();
                false
            }
            PasteResult::ClipboardHasNoText => {
                self.status = "Clipboard has no text".into();
                false
            }
        }
    }

    fn refresh_status(&mut self) {
        self.status = editor_commands::cursor_status(&self.core).into();
    }

    fn top_menu_index(id: TopMenuId) -> usize {
        TOP_MENUS.iter().position(|menu| menu.id == id).unwrap_or(0)
    }

    fn top_menu_by_id(id: TopMenuId) -> &'static TopMenu {
        TOP_MENUS
            .iter()
            .find(|menu| menu.id == id)
            .unwrap_or(&TOP_MENUS[0])
    }

    fn click_top_menu(&mut self, id: TopMenuId, cx: &mut Context<Self>) {
        if id == TopMenuId::Terminal {
            self.terminal_open = !self.terminal_open;
            self.status = if self.terminal_open {
                "Terminal panel opened".into()
            } else {
                "Terminal panel hidden".into()
            };
            self.open_menu = None;
            self.open_submenu = None;
            cx.notify();
            return;
        }

        if self.open_menu == Some(id) {
            self.open_menu = None;
            self.open_submenu = None;
        } else {
            self.open_menu = Some(id);
            self.open_submenu = None;
        }
        cx.notify();
    }

    fn hover_top_menu(&mut self, id: TopMenuId, cx: &mut Context<Self>) {
        if self.open_menu.is_some() && self.open_menu != Some(id) {
            self.open_menu = Some(id);
            self.open_submenu = None;
            cx.notify();
        }
    }

    fn hover_menu_item(&mut self, item: MenuItem, cx: &mut Context<Self>) {
        let next = if item.submenu.is_empty() {
            None
        } else {
            Some(item.id)
        };
        if self.open_submenu != next {
            self.open_submenu = next;
            cx.notify();
        }
    }

    fn close_menu_overlay(&mut self, cx: &mut Context<Self>) {
        if self.open_menu.is_some() || self.open_submenu.is_some() {
            self.open_menu = None;
            self.open_submenu = None;
            cx.notify();
        }
    }

    fn execute_menu_command(
        &mut self,
        cmd: MenuCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match AppMenuAction::from(cmd) {
            AppMenuAction::NewTextFile => {
                self.core.clear();
                self.hover_byte = None;
                self.status = "New text buffer created".into();
            }
            AppMenuAction::OpenFile => {
                self.status = "Open File is not implemented yet".into();
            }
            AppMenuAction::OpenFolder => self.open_project_dialog(window, cx),
            AppMenuAction::Save => self.save_active_file(cx),
            AppMenuAction::Exit => {
                self.status = "Exit is not implemented yet".into();
            }
            AppMenuAction::Undo => {
                self.undo_edit(cx);
            }
            AppMenuAction::Redo => {
                self.redo_edit(cx);
            }
            AppMenuAction::Cut => {
                let _ = self.cut_selection(cx);
            }
            AppMenuAction::Copy => {
                let _ = self.copy_selection(cx);
            }
            AppMenuAction::Paste => {
                let _ = self.paste_from_clipboard(cx);
            }
            AppMenuAction::Find => {
                self.status = "Find is not implemented yet".into();
            }
            AppMenuAction::Replace => {
                self.status = "Replace is not implemented yet".into();
            }
            AppMenuAction::SelectAll => {
                editor_commands::select_all(&mut self.core);
                self.refresh_status();
            }
            AppMenuAction::ExpandSelection => {
                editor_commands::expand_selection(&mut self.core);
                self.refresh_status();
            }
            AppMenuAction::CommandPalette => {
                self.status = "Command Palette is not implemented yet".into();
            }
            AppMenuAction::AppearancePanel => {
                self.status = "Appearance submenu opened".into();
            }
            AppMenuAction::ZoomIn => {
                self.status = "Zoom In is not implemented yet".into();
            }
            AppMenuAction::ZoomOut => {
                self.status = "Zoom Out is not implemented yet".into();
            }
            AppMenuAction::ZoomReset => {
                self.status = "Reset Zoom is not implemented yet".into();
            }
            AppMenuAction::ToggleTerminalPanel => {
                self.terminal_open = !self.terminal_open;
                self.status = if self.terminal_open {
                    "Terminal panel opened".into()
                } else {
                    "Terminal panel hidden".into()
                };
            }
            AppMenuAction::ClearTerminal => {
                self.terminal_lines.clear();
                self.terminal_lines.push("Terminal cleared".into());
                self.terminal_lines.push("PS D:\\Projects\\VeloCode>".into());
                self.status = "Terminal cleared".into();
            }
        }

        self.open_menu = None;
        self.open_submenu = None;
        cx.notify();
    }

    fn handle_editor_key(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.workspace.active_index.is_none() {
            return;
        }

        let action = resolve_editor_key_action(event);
        match action {
            EditorKeyAction::Save => {
                self.save_active_file(cx);
                return;
            }
            EditorKeyAction::Undo => {
                self.undo_edit(cx);
                return;
            }
            EditorKeyAction::Redo => {
                self.redo_edit(cx);
                return;
            }
            EditorKeyAction::Copy => {
                let _ = self.copy_selection(cx);
                cx.notify();
                return;
            }
            EditorKeyAction::Cut => {
                let changed = self.cut_selection(cx);
                if changed {
                    self.refresh_status();
                }
                cx.notify();
                return;
            }
            EditorKeyAction::Paste => {
                let changed = self.paste_from_clipboard(cx);
                if changed {
                    self.refresh_status();
                }
                cx.notify();
                return;
            }
            EditorKeyAction::SelectAll => {
                editor_commands::select_all(&mut self.core);
                self.refresh_status();
                cx.notify();
                return;
            }
            _ => {}
        }

        match editor_runtime::apply_core_key_action(&mut self.core, &action) {
            CoreKeyApply::Applied => {}
            CoreKeyApply::NotHandled => return,
        }

        self.hover_byte = None;
        self.refresh_status();
        cx.notify();
    }
    fn render_welcome(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let recent_items: Vec<(String, PathBuf)> = self
            .recent_projects
            .iter()
            .map(|path| {
                let label = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());
                (label, path.clone())
            })
            .collect();
        let menu_overlay = if let Some(open_id) = self.open_menu {
            let top_menu = Self::top_menu_by_id(open_id);
            let menu_index = Self::top_menu_index(open_id);
            let menu_left = 12.0 + (menu_index as f32 * MENU_BUTTON_WIDTH);
            let menu_top = MENU_BAR_HEIGHT;

            let mut submenu_rows: &'static [MenuItem] = &[];
            let mut submenu_top = menu_top;
            if let Some(open_submenu_id) = self.open_submenu {
                if let Some((item_idx, item)) = top_menu
                    .items
                    .iter()
                    .enumerate()
                    .find(|(_, item)| item.id == open_submenu_id)
                {
                    submenu_rows = item.submenu;
                    submenu_top = menu_top + (item_idx as f32 * MENU_ITEM_HEIGHT);
                }
            }

            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .bg(rgb(0x121212))
                        .opacity(0.01)
                        .id("menu-click-away")
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.close_menu_overlay(cx);
                        })),
                )
                .child(
                    div()
                        .id("menu-panel-main")
                        .absolute()
                        .top(px(menu_top))
                        .left(px(menu_left))
                        .w(px(MENU_PANEL_WIDTH))
                        .rounded_md()
                        .bg(rgb(0x181818))
                        .border_1()
                        .border_color(rgb(0x3C3C3C))
                        .py_1()
                        .flex_col()
                        .children(top_menu.items.iter().enumerate().map(|(idx, item)| {
                            let has_submenu = !item.submenu.is_empty();
                            let hovered = self.open_submenu == Some(item.id);
                            let row_item = *item;
                            div()
                                .id(("menu-main-row", idx))
                                .h(px(MENU_ITEM_HEIGHT))
                                .px_2()
                                .flex()
                                .items_center()
                                .justify_between()
                                .bg(if hovered { rgb(0x073A5A) } else { rgb(0x181818) })
                                .on_mouse_move(cx.listener(move |this, _: &MouseMoveEvent, _, cx| {
                                    this.hover_menu_item(row_item, cx);
                                }))
                                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                    if let Some(cmd) = row_item.command {
                                        this.execute_menu_command(cmd, window, cx);
                                    } else {
                                        this.hover_menu_item(row_item, cx);
                                    }
                                }))
                                .child(div().child(item.label))
                                .child(div().text_color(rgb(0x6F6F6F)).child(if has_submenu {
                                    ">"
                                } else {
                                    item.keybinding.unwrap_or("")
                                }))
                        })),
                )
                .child(if submenu_rows.is_empty() {
                    div().into_any_element()
                } else {
                    div()
                        .id("menu-panel-sub")
                        .absolute()
                        .top(px(submenu_top))
                        .left(px(menu_left + MENU_PANEL_WIDTH - 2.0))
                        .w(px(MENU_PANEL_WIDTH))
                        .rounded_md()
                        .bg(rgb(0x181818))
                        .border_1()
                        .border_color(rgb(0x3C3C3C))
                        .py_1()
                        .flex_col()
                        .children(submenu_rows.iter().enumerate().map(|(idx, item)| {
                            let row_item = *item;
                            div()
                                .id(("menu-sub-row", idx))
                                .h(px(MENU_ITEM_HEIGHT))
                                .px_2()
                                .flex()
                                .items_center()
                                .justify_between()
                                .bg(rgb(0x181818))
                                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                    if let Some(cmd) = row_item.command {
                                        this.execute_menu_command(cmd, window, cx);
                                    }
                                }))
                                .child(div().child(item.label))
                                .child(
                                    div()
                                        .text_color(rgb(0x6F6F6F))
                                        .child(item.keybinding.unwrap_or("")),
                                )
                        }))
                        .into_any_element()
                })
                .into_any_element()
        } else {
            div().into_any_element()
        };

        div()
            .size_full()
            .relative()
            .bg(rgb(0x121212))
            .text_color(rgb(0xCCCCCC))
            .font(Self::base_ui_font())
            .child(
                div()
                    .size_full()
                    .flex_col()
                    .child(
                        div()
                            .h(px(34.0))
                            .w_full()
                            .bg(rgb(0x181818))
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_4()
                                    .child("File")
                                    .child("Edit")
                                    .child("Selection")
                                    .child("View")
                                    .child("Go")
                                    .child("Run")
                                    .child("Terminal")
                                    .child("Help"),
                            )
                            .child(div().text_color(rgb(0x727272)).child("VeloCode")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .w(px(760.0))
                                    .max_w_full()
                                    .px_4()
                                    .py_4()
                                    .flex_col()
                                    .gap_3()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_3()
                                            .child(
                                                div()
                                                    .w(px(54.0))
                                                    .h(px(54.0))
                                                    .rounded_md()
                                                    .bg(rgb(0x1A1F2A))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .text_color(rgb(0xB8C0CE))
                                                    .child("V"),
                                            )
                                            .child(
                                                div()
                                                    .flex_col()
                                                    .gap_1()
                                                    .child("Welcome back to Velo")
                                                    .child(
                                                        div()
                                                            .text_color(rgb(0x9EA7B8))
                                                            .italic()
                                                            .child("The editor for what's next"),
                                                    ),
                                            ),
                                    )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_2()
                                                    .child(div().text_color(rgb(0x9EA7B8)).child("GET STARTED"))
                                                    .child(div().flex_1().h(px(1.0)).bg(rgb(0x2A2A2A))),
                                            )
                                            .child(
                                                div()
                                                    .flex_col()
                                                    .gap_1()
                                                    .child(
                                                        div()
                                                            .w_full()
                                                            .h(px(36.0))
                                                            .px_1()
                                                            .flex()
                                                            .items_center()
                                                            .justify_between()
                                                            .id("welcome-new-buffer")
                                                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                                                this.core.clear();
                                                                this.screen = Screen::Editor;
                                                                this.status = "New buffer".into();
                                                                cx.notify();
                                                            }))
                                                            .child(
                                                                div()
                                                                    .flex()
                                                                    .items_center()
                                                                    .gap_2()
                                                                    .child(img(self.icons.add()).size(px(14.0)))
                                                                    .child("New File"),
                                                            )
                                                            .child(div().text_color(rgb(0xA7B0C0)).child("Ctrl-N")),
                                                    )
                                                    .child(
                                                        div()
                                                            .w_full()
                                                            .h(px(36.0))
                                                            .px_1()
                                                            .flex()
                                                            .items_center()
                                                            .justify_between()
                                                            .id("welcome-open-folder")
                                                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                                                this.open_project_dialog(window, cx);
                                                            }))
                                                            .child(
                                                                div()
                                                                    .flex()
                                                                    .items_center()
                                                                    .gap_2()
                                                                    .child(img(self.icons.folder_open()).size(px(14.0)))
                                                                    .child("Open Project"),
                                                            )
                                                            .child(div().text_color(rgb(0xA7B0C0)).child("Ctrl-K  Ctrl-O")),
                                                    )
                                                    .child(
                                                        div()
                                                            .w_full()
                                                            .h(px(36.0))
                                                            .px_1()
                                                            .flex()
                                                            .items_center()
                                                            .justify_between()
                                                            .id("welcome-clone")
                                                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                                                this.status = "Clone Repository is not implemented yet".into();
                                                                cx.notify();
                                                            }))
                                                            .child(
                                                                div()
                                                                    .flex()
                                                                    .items_center()
                                                                    .gap_2()
                                                                    .child(">")
                                                                    .child("Clone Repository"),
                                                            )
                                                            .child(div().text_color(rgb(0xA7B0C0)).child("")),
                                                    )
                                                    .child(
                                                        div()
                                                            .w_full()
                                                            .h(px(36.0))
                                                            .px_1()
                                                            .flex()
                                                            .items_center()
                                                            .justify_between()
                                                            .id("welcome-command-palette")
                                                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                                                this.status = "Command Palette is not implemented yet".into();
                                                                cx.notify();
                                                            }))
                                                            .child(
                                                                div()
                                                                    .flex()
                                                                    .items_center()
                                                                    .gap_2()
                                                                    .child(">")
                                                                    .child("Open Command Palette"),
                                                            )
                                                            .child(div().text_color(rgb(0xA7B0C0)).child("Ctrl-Shift-P")),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .mt_2()
                                                    .flex()
                                                    .items_center()
                                                    .gap_2()
                                                    .child(div().text_color(rgb(0x9EA7B8)).child("RECENT PROJECTS"))
                                                    .child(div().flex_1().h(px(1.0)).bg(rgb(0x2A2A2A))),
                                            )
                                            .child(
                                                div()
                                                    .flex_col()
                                                    .gap_1()
                                                    .children(recent_items.iter().enumerate().map(|(idx, (item, path))| {
                                                        let recent_path = path.clone();
                                                        let item_font = Self::localized_ui_font(item);
                                                        div()
                                                            .w_full()
                                                            .h(px(36.0))
                                                            .px_1()
                                                            .flex()
                                                            .items_center()
                                                            .justify_between()
                                                            .id(("welcome-recent", idx))
                                                            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                                                if recent_path.is_dir() {
                                                                    this.load_project(recent_path.clone(), window, cx);
                                                                } else {
                                                                    this.status = format!(
                                                                        "Recent project not found: {}",
                                                                        recent_path.display()
                                                                    )
                                                                    .into();
                                                                    this.recent_projects.retain(|p| p != &recent_path);
                                                                    this.persist_recent_projects();
                                                                    cx.notify();
                                                                }
                                                            }))
                                                            .child(
                                                                div()
                                                                    .flex()
                                                                    .items_center()
                                                                    .gap_2()
                                                                    .child(img(self.icons.folder()).size(px(14.0)))
                                                                    .child(div().font(item_font).child(item.clone())),
                                                            )
                                                            .child(div().text_color(rgb(0xA7B0C0)).child(format!("Ctrl-{}", idx + 1)))
                                                    }))
                                                    .child(if recent_items.is_empty() {
                                                        div()
                                                            .h(px(30.0))
                                                            .px_1()
                                                            .text_color(rgb(0x6F6F6F))
                                                            .child("No recent projects yet")
                                                            .into_any_element()
                                                    } else {
                                                        div().into_any_element()
                                                    }),
                                            ),
                            ),
                    ),
            )
            .child(menu_overlay)
            .into_any_element()
    }
    fn render_workspace(&mut self, cx: &mut Context<Self>, window: &mut Window) -> AnyElement {
        let viewport_w = f32::from(window.viewport_size().width);
        let viewport_h = f32::from(window.viewport_size().height);
        self.last_viewport_width = viewport_w;
        self.clamp_sidebar_width();
        let entries = self.workspace.visible_entries();
        let explorer_view = compute_explorer_view(entries.len(), viewport_h, self.explorer_scroll);
        self.explorer_scroll = explorer_view.scroll;

        // Fallback recovery: if active tab points to a non-empty file but editor buffer is empty,
        // reload text directly from disk so rendering can recover without reopening the project.
        if self.core.text.is_empty() {
            if let Some(active_idx) = self.workspace.active_index {
                if let Some(file) = self.workspace.files.get(active_idx) {
                    if let Ok(raw) = fs::read_to_string(&file.abs_path) {
                        let normalized = raw.replace("\r\n", "\n").replace('\r', "\n");
                        if !normalized.is_empty() {
                            self.core.set_text(normalized);
                            self.status = format!("Recovered view: {}", file.rel_path).into();
                        }
                    } else if let Ok(decoded) = decode_text_file(&file.abs_path) {
                        let normalized = decoded.text.replace("\r\n", "\n").replace('\r', "\n");
                        if !normalized.is_empty() {
                            self.core.set_text(normalized);
                            self.status = format!("Recovered view: {}", file.rel_path).into();
                        }
                    }
                }
            }
        }

        let project_name: SharedString = self
            .workspace
            .project_root
            .as_ref()
            .and_then(|p| {
                p.file_name()
                    .map(|n| n.to_string_lossy().to_string().into())
            })
            .unwrap_or_else(|| "Workspace".into());

        let active_language = self
            .workspace
            .active_index
            .map(|idx| self.workspace.files[idx].language)
            .unwrap_or("text");
        let mut editor_view = compute_editor_view(
            &self.core.text,
            viewport_w,
            viewport_h,
            self.sidebar_width,
            EDITOR_LINE_HEIGHT,
            EDITOR_GLYPH_WIDTH,
            self.editor_scroll,
            self.editor_hscroll,
        );

        let core_has_visible_text = self.core.text.chars().any(|c| !c.is_whitespace());
        let viewport_has_visible_text = editor_view.viewport_text.chars().any(|c| !c.is_whitespace());
        if core_has_visible_text && !viewport_has_visible_text {
            self.editor_scroll = 0.0;
            self.editor_hscroll = 0.0;
            editor_view = compute_editor_view(
                &self.core.text,
                viewport_w,
                viewport_h,
                self.sidebar_width,
                EDITOR_LINE_HEIGHT,
                EDITOR_GLYPH_WIDTH,
                self.editor_scroll,
                self.editor_hscroll,
            );
        }
        self.editor_scroll = editor_view.scroll;
        self.editor_hscroll = editor_view.hscroll;
        let line_count = editor_view.line_count;
        let mut editor_text_to_render = editor_view.viewport_text.clone();
        let mut editor_line_numbers_to_render = editor_view.line_numbers.clone();
        let mut use_full_buffer_fallback = false;
        if !self.core.text.is_empty()
            && (editor_text_to_render.is_empty()
                || editor_text_to_render
                    .lines()
                    .all(|line| line.trim().is_empty()))
        {
            use_full_buffer_fallback = true;
            self.editor_scroll = 0.0;
            self.editor_hscroll = 0.0;
            editor_text_to_render = self.core.text.clone();
            let full_line_count = self.core.text.split('\n').count().max(1);
            let line_number_width = full_line_count.to_string().len().max(2);
            editor_line_numbers_to_render = (1..=full_line_count)
                .map(|line| format!("{:>width$}", line, width = line_number_width))
                .collect::<Vec<_>>()
                .join("\n");
        }

        let editor_plain_text: SharedString = editor_text_to_render.clone().into();
        let editor_line_numbers: SharedString = editor_line_numbers_to_render.into();

        let text_layout = TextLayout::from_text(&self.core.text);
        let selection_ranges = if use_full_buffer_fallback {
            self.core
                .selection_byte_range()
                .map(|(start, end)| vec![start..end])
                .unwrap_or_default()
        } else {
            selection::selection_byte_ranges_in_viewport(
                &self.core.selection,
                &text_layout,
                ScrollOffset {
                    line: editor_view.start_line,
                    column: editor_view.start_col,
                },
                ViewportCells {
                    rows: editor_view.visible_rows,
                    cols: editor_view.visible_cols,
                },
                &editor_view.viewport_text,
            )
        };

        let has_selection = !selection_ranges.is_empty();
        let selection_overlay = StyledText::new(editor_text_to_render).with_highlights(
            selection_ranges.into_iter().map(|range| {
                (
                    range,
                    HighlightStyle {
                        background_color: Some(rgb(0x1F3F5E).into()),
                        ..Default::default()
                    },
                )
            }),
        );
        let terminal_text: SharedString = self
            .terminal_lines
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join("\n")
            .into();
        let status_font = Self::localized_ui_font(self.status.as_ref());
        let (editor_thumb_h, editor_thumb_top, editor_scrollable) = scrollbar_metrics(
            editor_view.line_count,
            editor_view.visible_rows,
            editor_view.start_line,
            editor_view.track_h,
        );
        let (editor_hthumb_w, editor_hthumb_left, editor_hscrollable) = scrollbar_metrics(
            editor_view.max_line_cols.max(1),
            editor_view.visible_cols,
            editor_view.start_col,
            editor_view.htrack_w,
        );
        let visible_entries = &entries[explorer_view.start..explorer_view.end];
        let panel_title = match self.active_panel {
            ActivityPanel::Explorer => "Explorer",
            ActivityPanel::Search => "Search",
            ActivityPanel::SourceControl => "Source Control",
            ActivityPanel::Run => "Run and Debug",
            ActivityPanel::Extensions => "Extensions",
            ActivityPanel::Settings => "Settings",
        };
        let editor_area_width = (viewport_w - self.sidebar_width - 110.0).max(240.0);
        let tab_visible_count = ((editor_area_width - 72.0) / 154.0).floor() as usize;
        let tab_visible_count = tab_visible_count.clamp(1, 12);
        let max_tab_scroll = self
            .workspace
            .open_tabs
            .len()
            .saturating_sub(tab_visible_count);
        self.tab_scroll = self.tab_scroll.min(max_tab_scroll);
        let tab_end = (self.tab_scroll + tab_visible_count).min(self.workspace.open_tabs.len());
        let visible_tabs = &self.workspace.open_tabs[self.tab_scroll..tab_end];
        let total = entries.len().max(1);
        let (thumb_h, thumb_top, explorer_scrollable) = scrollbar_metrics(
            total,
            explorer_view.visible_rows,
            explorer_view.start,
            explorer_view.track_h,
        );
        let sidebar_body = if self.active_panel == ActivityPanel::Explorer {
            div()
                .flex()
                .gap_2()
                .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                    this.scroll_explorer(event, cx);
                }))
                .child(
                    div().flex_1().flex_col().gap_1().children(
                        visible_entries.iter().enumerate().map(|(visible_idx, row)| {
                            let row_id = explorer_view.start + visible_idx;
                            match &row.kind {
                                VisibleKind::Folder {
                                    abs_path,
                                    name,
                                    expanded,
                                } => {
                                    let folder = abs_path.clone();
                                    let icon = if *expanded {
                                        self.icons.folder_open()
                                    } else {
                                        self.icons.folder()
                                    };
                                    div()
                                        .flex()
                                        .items_center()
                                        .overflow_hidden()
                                        .gap_2()
                                        .px_2()
                                        .py(px(2.0))
                                        .rounded_sm()
                                        .bg(rgb(0x121212))
                                        .id(("explorer-folder", row_id))
                                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                            this.workspace.toggle_folder(&folder);
                                            cx.notify();
                                        }))
                                        .child(div().w(px((row.depth as f32) * 14.0)))
                                        .child(img(icon).size(px(20.0)))
                                        .child(div().flex_1().truncate().child(name.clone()))
                                }
                                VisibleKind::File { file_idx } => {
                                    let idx = *file_idx;
                                    let file = &self.workspace.files[idx];
                                    let selected = self.workspace.active_index == Some(idx);
                                    div()
                                        .flex()
                                        .items_center()
                                        .overflow_hidden()
                                        .gap_2()
                                        .px_2()
                                        .py(px(2.0))
                                        .rounded_sm()
                                        .bg(if selected { rgb(0x073A5A) } else { rgb(0x121212) })
                                        .id(("explorer-file", row_id))
                                        .on_click(cx.listener(
                                            move |this, _: &ClickEvent, window, cx| {
                                                this.open_file_at(idx, window, cx);
                                            },
                                        ))
                                        .child(div().w(px((row.depth as f32) * 14.0)))
                                        .child(img(this_icon(&self.icons, file)).size(px(20.0)))
                                        .child(div().flex_1().truncate().child(file.name.clone()))
                                }
                            }
                        }),
                    ),
                )
                .child(
                    div()
                        .w(px(8.0))
                        .h(px(explorer_view.track_h))
                        .rounded_md()
                        .bg(rgb(0x1B1D1E))
                        .child(
                            div()
                                .w(px(8.0))
                                .h(px(thumb_h))
                                .mt(px(thumb_top))
                                .rounded_md()
                                .bg(if explorer_scrollable {
                                    rgb(0x6F6F6F)
                                } else {
                                    rgb(0x1B1D1E)
                                }),
                        ),
                )
                .into_any_element()
        } else {
            div()
                .flex_1()
                .rounded_sm()
                .bg(rgb(0x121212))
                .p_3()
                .flex_col()
                .gap_2()
                .child(div().text_color(rgb(0xCCCCCC)).child(panel_title))
                .child(
                    div().text_color(rgb(0x6F6F6F)).child(match self.active_panel {
                        ActivityPanel::Search => {
                            "Search view placeholder: file search will appear here."
                        }
                        ActivityPanel::SourceControl => {
                            "Source Control view placeholder: changes and commits."
                        }
                        ActivityPanel::Run => {
                            "Run and Debug view placeholder: launch configs and sessions."
                        }
                        ActivityPanel::Extensions => {
                            "Extensions view placeholder: installed/marketplace extensions."
                        }
                        ActivityPanel::Settings => {
                            "Settings view placeholder: preferences and keybindings."
                        }
                        ActivityPanel::Explorer => "",
                    }),
                )
                .into_any_element()
        };
        let menu_overlay = if let Some(open_id) = self.open_menu {
            let top_menu = Self::top_menu_by_id(open_id);
            let menu_index = Self::top_menu_index(open_id);
            let menu_left = 12.0 + (menu_index as f32 * MENU_BUTTON_WIDTH);
            let menu_top = MENU_BAR_HEIGHT;

            let mut submenu_rows: &'static [MenuItem] = &[];
            let mut submenu_top = menu_top;
            if let Some(open_submenu_id) = self.open_submenu {
                if let Some((item_idx, item)) = top_menu
                    .items
                    .iter()
                    .enumerate()
                    .find(|(_, item)| item.id == open_submenu_id)
                {
                    submenu_rows = item.submenu;
                    submenu_top = menu_top + (item_idx as f32 * MENU_ITEM_HEIGHT);
                }
            }

            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .bg(rgb(0x121212))
                        .opacity(0.01)
                        .id("menu-click-away")
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.close_menu_overlay(cx);
                        })),
                )
                .child(
                    div()
                        .id("menu-panel-main")
                        .absolute()
                        .top(px(menu_top))
                        .left(px(menu_left))
                        .w(px(MENU_PANEL_WIDTH))
                        .rounded_md()
                        .bg(rgb(0x181818))
                        .border_1()
                        .border_color(rgb(0x3C3C3C))
                        .py_1()
                        .flex_col()
                        .children(top_menu.items.iter().enumerate().map(|(idx, item)| {
                            let has_submenu = !item.submenu.is_empty();
                            let hovered = self.open_submenu == Some(item.id);
                            let row_item = *item;
                            div()
                                .id(("menu-main-row", idx))
                                .h(px(MENU_ITEM_HEIGHT))
                                .px_2()
                                .flex()
                                .items_center()
                                .justify_between()
                                .bg(if hovered { rgb(0x073A5A) } else { rgb(0x181818) })
                                .on_mouse_move(cx.listener(move |this, _: &MouseMoveEvent, _, cx| {
                                    this.hover_menu_item(row_item, cx);
                                }))
                                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                    if let Some(cmd) = row_item.command {
                                        this.execute_menu_command(cmd, window, cx);
                                    } else {
                                        this.hover_menu_item(row_item, cx);
                                    }
                                }))
                                .child(div().child(item.label))
                                .child(div().text_color(rgb(0x6F6F6F)).child(if has_submenu {
                                    ">"
                                } else {
                                    item.keybinding.unwrap_or("")
                                }))
                        })),
                )
                .child(if submenu_rows.is_empty() {
                    div().into_any_element()
                } else {
                    div()
                        .id("menu-panel-sub")
                        .absolute()
                        .top(px(submenu_top))
                        .left(px(menu_left + MENU_PANEL_WIDTH - 2.0))
                        .w(px(MENU_PANEL_WIDTH))
                        .rounded_md()
                        .bg(rgb(0x181818))
                        .border_1()
                        .border_color(rgb(0x3C3C3C))
                        .py_1()
                        .flex_col()
                        .children(submenu_rows.iter().enumerate().map(|(idx, item)| {
                            let row_item = *item;
                            div()
                                .id(("menu-sub-row", idx))
                                .h(px(MENU_ITEM_HEIGHT))
                                .px_2()
                                .flex()
                                .items_center()
                                .justify_between()
                                .bg(rgb(0x181818))
                                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                    if let Some(cmd) = row_item.command {
                                        this.execute_menu_command(cmd, window, cx);
                                    }
                                }))
                                .child(div().child(item.label))
                                .child(
                                    div()
                                        .text_color(rgb(0x6F6F6F))
                                        .child(item.keybinding.unwrap_or("")),
                                )
                        }))
                        .into_any_element()
                })
                .into_any_element()
        } else {
            div().into_any_element()
        };

        div()
            .size_full()
            .relative()
            .bg(rgb(0x121212))
            .text_color(rgb(0xCCCCCC))
            .font(Self::base_ui_font())
            .child(
                div()
                    .size_full()
                    .flex_col()
                    .child(
                        div()
                            .h(px(MENU_BAR_HEIGHT))
                            .w_full()
                            .px_3()
                            .flex()
                            .items_center()
                            .gap_0()
                            .bg(rgb(0x181818))
                            .text_color(rgb(0x727272))
                            .children(TOP_MENUS.iter().enumerate().map(|(menu_idx, menu)| {
                                let is_open = self.open_menu == Some(menu.id);
                                div()
                                    .id(("menubar-item", menu_idx))
                                    .w(px(MENU_BUTTON_WIDTH))
                                    .h(px(24.0))
                                    .px_1()
                                    .rounded_sm()
                                    .flex()
                                    .items_center()
                                    .justify_start()
                                    .bg(if is_open { rgb(0x202B3A) } else { rgb(0x181818) })
                                    .on_mouse_move(cx.listener({
                                        let id = menu.id;
                                        move |this, _: &MouseMoveEvent, _, cx| {
                                            this.hover_top_menu(id, cx);
                                        }
                                    }))
                                    .on_click(cx.listener({
                                        let id = menu.id;
                                        move |this, _: &ClickEvent, _, cx| {
                                            this.click_top_menu(id, cx);
                                        }
                                    }))
                                    .child(menu.label)
                            })),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                    .child(
                        div()
                            .w(px(72.0))
                            .h_full()
                            .bg(rgb(0x121212))
                            .flex_col()
                            .justify_between()
                            .py_0()
                            .child(
                                div()
                                    .w_full()
                                    .flex_col()
                                    .gap_0()
                                    .child(
                                        div()
                                            .w_full()
                                            .h(px(56.0))
                                            .bg(if self.active_panel == ActivityPanel::Explorer {
                                                rgb(0x073A5A)
                                            } else {
                                                rgb(0x181818)
                                            })
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .id("activity-explorer")
                                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                                this.set_active_panel(ActivityPanel::Explorer, cx);
                                            }))
                                            .child(img(self.icons.activity_explorer()).size(px(28.0))),
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .h(px(56.0))
                                            .bg(if self.active_panel == ActivityPanel::Search {
                                                rgb(0x073A5A)
                                            } else {
                                                rgb(0x181818)
                                            })
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .id("activity-search")
                                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                                this.set_active_panel(ActivityPanel::Search, cx);
                                            }))
                                            .child(img(self.icons.activity_search()).size(px(28.0))),
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .h(px(56.0))
                                            .bg(if self.active_panel == ActivityPanel::SourceControl {
                                                rgb(0x073A5A)
                                            } else {
                                                rgb(0x181818)
                                            })
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .id("activity-source-control")
                                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                                this.set_active_panel(ActivityPanel::SourceControl, cx);
                                            }))
                                            .child(
                                                img(self.icons.activity_source_control()).size(px(28.0)),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .h(px(56.0))
                                            .bg(if self.active_panel == ActivityPanel::Run {
                                                rgb(0x073A5A)
                                            } else {
                                                rgb(0x181818)
                                            })
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .id("activity-run")
                                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                                this.set_active_panel(ActivityPanel::Run, cx);
                                            }))
                                            .child(img(self.icons.activity_run()).size(px(28.0))),
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .h(px(56.0))
                                            .bg(if self.active_panel == ActivityPanel::Extensions {
                                                rgb(0x073A5A)
                                            } else {
                                                rgb(0x181818)
                                            })
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .id("activity-extensions")
                                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                                this.set_active_panel(ActivityPanel::Extensions, cx);
                                            }))
                                            .child(img(self.icons.activity_extensions()).size(px(28.0))),
                                    ),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .flex_col()
                                    .gap_0()
                                    .child(
                                        div()
                                            .w_full()
                                            .h(px(56.0))
                                            .bg(if self.active_panel == ActivityPanel::Settings {
                                                rgb(0x073A5A)
                                            } else {
                                                rgb(0x181818)
                                            })
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .id("activity-settings")
                                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                                this.set_active_panel(ActivityPanel::Settings, cx);
                                            }))
                                            .child(img(self.icons.settings()).size(px(28.0))),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .w(px((self.sidebar_width - 58.0).max(150.0)))
                            .h_full()
                            .p_2()
                            .overflow_hidden()
                            .bg(rgb(0x181818))
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(img(self.icons.folder()).size(px(17.0)))
                                    .child(project_name),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py(px(2.0))
                                    .rounded_sm()
                                    .bg(rgb(0x121212))
                                    .text_color(rgb(0x3794FF))
                                    .child(panel_title),
                            )
                            .child(sidebar_body),
                    )
                    .child(
                        div()
                            .w(px(6.0))
                            .h_full()
                            .bg(rgb(0x1B1D1E))
                            .id("sidebar-splitter")
                            .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                                if event.click_count() >= 2 {
                                    this.reset_sidebar_width(cx);
                                }
                            }))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, event: &MouseDownEvent, _, cx| {
                                    this.start_sidebar_resize(event);
                                    cx.notify();
                                }),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .p_2()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .w(px(28.0))
                                            .h(px(28.0))
                                            .rounded_md()
                                            .bg(rgb(0x121212))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .id("tabs-left")
                                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                                this.tab_scroll = this.tab_scroll.saturating_sub(1);
                                                cx.notify();
                                            }))
                                            .child(img(self.icons.back()).size(px(14.0))),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .overflow_hidden()
                                            .flex()
                                            .gap_1()
                                            .children(visible_tabs.iter().map(|tab_idx| {
                                                let file = &self.workspace.files[*tab_idx];
                                                let selected = self.workspace.active_index == Some(*tab_idx);
                                                let mut label = Self::compact_label(&file.rel_path, 22).to_string();
                                                if selected && self.core.is_dirty {
                                                    label.push_str(" *");
                                                }
                                                div()
                                                    .flex_shrink_0()
                                                    .flex()
                                                    .flex_col()
                                                    .items_center()
                                                    .overflow_hidden()
                                                    .w(px(150.0))
                                                    .rounded_md()
                                                    .bg(if selected { rgb(0x073A5A) } else { rgb(0x121212) })
                                                    .id(("tab", *tab_idx))
                                                    .child(
                                                        div()
                                                            .w_full()
                                                            .flex()
                                                            .items_center()
                                                            .gap_2()
                                                            .px_3()
                                                            .py_1()
                                                            .child(img(this_icon(&self.icons, file)).size(px(16.0)))
                                                            .child(
                                                                div()
                                                                    .flex_1()
                                                                    .truncate()
                                                                    .id(("tab-open", *tab_idx))
                                                                    .on_click(cx.listener({
                                                                        let tab_idx = *tab_idx;
                                                                        move |this, _: &ClickEvent, window, cx| {
                                                                            this.open_file_at(tab_idx, window, cx);
                                                                        }
                                                                    }))
                                                                    .child(label),
                                                            )
                                                            .child(
                                                                div()
                                                                    .w(px(18.0))
                                                                    .h(px(18.0))
                                                                    .rounded_sm()
                                                                    .bg(if selected { rgb(0x073A5A) } else { rgb(0x121212) })
                                                                    .flex()
                                                                    .items_center()
                                                                    .justify_center()
                                                                    .id(("tab-close", *tab_idx))
                                                                    .on_click(cx.listener({
                                                                        let tab_idx = *tab_idx;
                                                                        move |this, _: &ClickEvent, _, cx| {
                                                                            this.close_file_tab(tab_idx, cx);
                                                                        }
                                                                    }))
                                                                    .child(img(self.icons.close_tab()).size(px(10.0))),
                                                            ),
                                                    )
                                                    .child(
                                                        div()
                                                            .w_full()
                                                            .h(px(2.0))
                                                            .bg(if selected { rgb(0x3794FF) } else { rgb(0x121212) }),
                                                    )
                                            })),
                                    )
                                    .child(
                                        div()
                                            .w(px(28.0))
                                            .h(px(28.0))
                                            .rounded_md()
                                            .bg(rgb(0x121212))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .id("tabs-right")
                                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                                this.tab_scroll = this.tab_scroll.saturating_add(1);
                                                cx.notify();
                                            }))
                                            .child(img(self.icons.next()).size(px(14.0))),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .rounded_md()
                                    .bg(rgb(0x121212))
                                    .p_2()
                                    .flex_col()
                                    .gap_1()
                                    .track_focus(&self.editor_focus)
                                    .id("editor-surface")
                                    .on_click(cx.listener(|this, _: &ClickEvent, window, _| {
                                        window.focus(&this.editor_focus);
                                    }))
                                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                                        this.handle_editor_key(event, window, cx);
                                    }))
                                    .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                                        this.scroll_editor(event, cx);
                                    }))
                                    .child(
                                        div()
                                            .flex_1()
                                            .flex()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .rounded_sm()
                                                    .bg(rgb(0x181818))
                                                    .p_2()
                                                    .overflow_hidden()
                                                    .child(
                                                        div()
                                                            .size_full()
                                                            .relative()
                                                            .flex()
                                                            .gap_2()
                                                            .child(
                                                                div()
                                                                    .w(px(52.0))
                                                                    .h_full()
                                                                    .bg(rgb(0x121212))
                                                                    .text_color(rgb(0x727272))
                                                                    .text_right()
                                                                    .px_2()
                                                                    .child(editor_line_numbers),
                                                            )
                                                            .child(
                                                                div()
                                                                    .flex_1()
                                                                    .h_full()
                                                                    .overflow_hidden()
                                                                    .relative()
                                                                    .on_mouse_down(
                                                                        MouseButton::Left,
                                                                        cx.listener(
                                                                            |this, event: &MouseDownEvent, window, cx| {
                                                                                window.focus(&this.editor_focus);
                                                                                this.begin_selection_drag(event, window, cx);
                                                                            },
                                                                        ),
                                                                    )
                                                                    .on_mouse_move(cx.listener(
                                                                        |this, event: &MouseMoveEvent, window, cx| {
                                                                            this.update_selection_drag(event, window, cx);
                                                                        },
                                                                    ))
                                                                    .on_mouse_up(
                                                                        MouseButton::Left,
                                                                        cx.listener(
                                                                            |this, event: &MouseUpEvent, window, cx| {
                                                                                this.end_selection_drag(event, window, cx);
                                                                            },
                                                                        ),
                                                                    )
                                                                    .child(
                                                                        div()
                                                                            .absolute()
                                                                            .top_0()
                                                                            .left_0()
                                                                            .size_full()
                                                                            .text_color(rgb(0xD4D4D4))
                                                                            .font_family("Consolas")
                                                                            .child(editor_plain_text.clone()),
                                                                    )
                                                                    .child(if has_selection {
                                                                        div()
                                                                            .absolute()
                                                                            .top_0()
                                                                            .left_0()
                                                                            .size_full()
                                                                            .text_color(rgb(0xD4D4D4))
                                                                            .font_family("Consolas")
                                                                            .child(selection_overlay)
                                                                            .into_any_element()
                                                                    } else {
                                                                        div().into_any_element()
                                                                    }),
                                                            ),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .w(px(10.0))
                                                    .h(px(editor_view.track_h))
                                                    .rounded_md()
                                                    .bg(rgb(0x1B1D1E))
                                                    .child(
                                                        div()
                                                            .w(px(10.0))
                                                            .h(px(editor_thumb_h))
                                                            .mt(px(editor_thumb_top))
                                                            .rounded_md()
                                                            .bg(if editor_scrollable {
                                                                rgb(0x666666)
                                                            } else {
                                                                rgb(0x1B1D1E)
                                                            }),
                                                    ),
                                            )
                                    )
                                    .child(
                                        div()
                                            .h(px(10.0))
                                            .w_full()
                                            .rounded_md()
                                            .bg(rgb(0x1B1D1E))
                                            .child(
                                                div()
                                                    .h(px(10.0))
                                                    .w(px(editor_hthumb_w))
                                                    .ml(px(editor_hthumb_left))
                                                    .rounded_md()
                                                    .bg(if editor_hscrollable {
                                                        rgb(0x666666)
                                                    } else {
                                                        rgb(0x1B1D1E)
                                                    }),
                                            ),
                                    ),
                            )
                            .child(if self.terminal_open {
                                div()
                                    .h(px(self.terminal_height))
                                    .rounded_md()
                                    .overflow_hidden()
                                    .bg(rgb(0x0F1116))
                                    .border_t_1()
                                    .border_color(rgb(0x2A2A2A))
                                    .flex_col()
                                    .child(
                                        div()
                                            .h(px(32.0))
                                            .px_3()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .bg(rgb(0x161A22))
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_2()
                                                    .child(img(self.icons.terminal()).size(px(14.0)))
                                                    .child(div().text_color(rgb(0xC5C5C5)).child("Terminal")),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_2()
                                                    .child(
                                                        div()
                                                            .w(px(22.0))
                                                            .h(px(22.0))
                                                            .rounded_sm()
                                                            .flex()
                                                            .items_center()
                                                            .justify_center()
                                                            .bg(rgb(0x121212))
                                                            .id("terminal-run")
                                                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                                                this.terminal_lines.push("PS D:\\Projects\\VeloCode> running (stub)".into());
                                                                this.status = "Terminal run invoked".into();
                                                                cx.notify();
                                                            }))
                                                            .child(img(self.icons.terminal_run()).size(px(11.0))),
                                                    )
                                                    .child(
                                                        div()
                                                            .w(px(22.0))
                                                            .h(px(22.0))
                                                            .rounded_sm()
                                                            .flex()
                                                            .items_center()
                                                            .justify_center()
                                                            .bg(rgb(0x121212))
                                                            .id("terminal-clear")
                                                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                                                this.terminal_lines.clear();
                                                                this.terminal_lines.push("Terminal cleared".into());
                                                                this.terminal_lines.push("PS D:\\Projects\\VeloCode>".into());
                                                                this.status = "Terminal cleared".into();
                                                                cx.notify();
                                                            }))
                                                            .child(img(self.icons.terminal_clear()).size(px(11.0))),
                                                    )
                                                    .child(
                                                        div()
                                                            .w(px(22.0))
                                                            .h(px(22.0))
                                                            .rounded_sm()
                                                            .flex()
                                                            .items_center()
                                                            .justify_center()
                                                            .bg(rgb(0x121212))
                                                            .id("terminal-close")
                                                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                                                this.terminal_open = false;
                                                                this.status = "Terminal panel hidden".into();
                                                                cx.notify();
                                                            }))
                                                            .child(img(self.icons.close_tab()).size(px(11.0))),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .overflow_hidden()
                                            .px_3()
                                            .py_2()
                                            .text_color(rgb(0xC9D1D9))
                                            .font_family("Consolas")
                                            .child(terminal_text),
                                    )
                                    .into_any_element()
                            } else {
                                div().into_any_element()
                            })
                            .child(
                                div()
                                    .h(px(24.0))
                                    .rounded_md()
                                    .px_2()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .bg(rgb(0x121212))
                                    .text_color(rgb(0x6F6F6F))
                                    .child(div().font(status_font).child(self.status.clone()))
                                    .child(format!("{} lines | {} | GPUI", line_count, active_language)),
                            ),
                    ),
                    ),
            )
            .child(menu_overlay)
            .id("workspace-root")
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                this.drag_sidebar_resize(event, cx);
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _, cx| {
                    this.stop_sidebar_resize(event, cx);
                }),
            )
            .on_click(cx.listener(|this, _: &ClickEvent, window, _| {
                if this.workspace.active_index.is_some() {
                    window.focus(&this.editor_focus);
                }
            }))
            .into_any_element()
    }
}

fn this_icon(icons: &Icons, entry: &FileEntry) -> PathBuf {
    icons.by_name(entry.icon_name)
}

impl Render for VeloIde {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match self.screen {
            Screen::Welcome => self.render_welcome(cx),
            Screen::Editor => self.render_workspace(cx, window),
        }
    }
}




