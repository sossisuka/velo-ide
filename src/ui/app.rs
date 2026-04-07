use crate::ui::{
    highlight::syntax_highlighted_text,
    language::language_and_icon_for,
    selection::{self, ScrollOffset, SelectionState, TextLayout, TextMetrics, ViewportCells},
};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use gpui::{
    div, img, px, rgb, AnyElement, ClickEvent, Context, FocusHandle,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement, Render, ScrollDelta, ScrollWheelEvent,
    SharedString, StatefulInteractiveElement, Styled, Window,
};

#[derive(Clone)]
pub struct Icons {
    base: PathBuf,
}

impl Icons {
    pub fn from_dir(dir: &Path) -> Self {
        Self {
            base: dir.to_path_buf(),
        }
    }

    fn by_name(&self, name: &str) -> PathBuf {
        self.base.join(format!("{name}.svg"))
    }

    fn file(&self) -> PathBuf {
        self.by_name("file")
    }

    fn folder(&self) -> PathBuf {
        self.by_name("folder")
    }

    fn folder_open(&self) -> PathBuf {
        self.by_name("folder_open")
    }

    fn run(&self) -> PathBuf {
        self.by_name("cli")
    }

    fn settings(&self) -> PathBuf {
        self.by_name("editorconfig")
    }
}

#[derive(Clone)]
struct FileEntry {
    abs_path: PathBuf,
    rel_path: SharedString,
    name: SharedString,
    language: &'static str,
    icon_name: &'static str,
}

#[derive(Clone)]
enum NodeKind {
    Folder { children: Vec<TreeNode> },
    File { file_idx: usize },
}

#[derive(Clone)]
struct TreeNode {
    abs_path: PathBuf,
    name: SharedString,
    kind: NodeKind,
}

#[derive(Clone)]
enum VisibleKind {
    Folder { abs_path: PathBuf, name: SharedString, expanded: bool },
    File { file_idx: usize },
}

#[derive(Clone)]
struct VisibleEntry {
    depth: usize,
    kind: VisibleKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Welcome,
    Editor,
}

const EDITOR_LINE_HEIGHT: f32 = 18.0;
const EDITOR_GLYPH_WIDTH: f32 = 8.2;
const EDITOR_LEFT_PADDING: f32 = 8.0;
const EDITOR_TOP_PADDING: f32 = 8.0;
const EDITOR_GUTTER_WIDTH: f32 = 60.0;

pub struct VeloIde {
    icons: Icons,
    screen: Screen,
    editor_focus: FocusHandle,

    project_root: Option<PathBuf>,
    files: Vec<FileEntry>,
    tree: Vec<TreeNode>,
    expanded_folders: HashSet<PathBuf>,
    open_tabs: Vec<usize>,
    active_index: Option<usize>,
    sidebar_width: f32,
    resizing_sidebar: bool,
    resize_start_x: f32,
    resize_start_width: f32,
    explorer_scroll: f32,
    editor_scroll: f32,
    editor_hscroll: f32,

    editor_text: String,
    cursor_byte: usize,
    selection: SelectionState,
    hover_byte: Option<usize>,
    is_dirty: bool,
    status: SharedString,
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
            chars.push('…');
        }
        chars.into_iter().collect::<String>().into()
    }

    pub fn new(icons: Icons, cx: &mut Context<Self>) -> Self {
        Self {
            icons,
            screen: Screen::Welcome,
            editor_focus: cx.focus_handle(),
            project_root: None,
            files: Vec::new(),
            tree: Vec::new(),
            expanded_folders: HashSet::new(),
            open_tabs: Vec::new(),
            active_index: None,
            sidebar_width: 300.0,
            resizing_sidebar: false,
            resize_start_x: 0.0,
            resize_start_width: 300.0,
            explorer_scroll: 0.0,
            editor_scroll: 0.0,
            editor_hscroll: 0.0,
            editor_text: String::new(),
            cursor_byte: 0,
            selection: SelectionState::default(),
            hover_byte: None,
            is_dirty: false,
            status: "Ready".into(),
        }
    }

    fn open_project_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(folder) = rfd::FileDialog::new().set_title("Open Project Folder").pick_folder() {
            self.load_project(folder, window, cx);
        }
    }

    fn load_project(&mut self, root: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let mut acc = Vec::new();
        let tree = Self::build_tree(&root, &root, &mut acc, 0, 4000);

        self.project_root = Some(root.clone());
        self.files = acc;
        self.tree = tree;
        self.expanded_folders.clear();
        self.expanded_folders.insert(root.clone());
        self.open_tabs.clear();
        self.active_index = None;
        self.editor_text.clear();
        self.cursor_byte = 0;
        self.selection.clear();
        self.hover_byte = None;
        self.editor_hscroll = 0.0;
        self.is_dirty = false;
        self.screen = Screen::Editor;
        self.status = format!(
            "Opened project: {} ({} files)",
            root.display(),
            self.files.len()
        )
        .into();

        if !self.files.is_empty() {
            self.open_file_at(0, window, cx);
        }

        cx.notify();
    }

    fn build_tree(
        base: &Path,
        dir: &Path,
        files: &mut Vec<FileEntry>,
        depth: usize,
        max_files: usize,
    ) -> Vec<TreeNode> {
        if depth > 32 || files.len() >= max_files {
            return Vec::new();
        }

        let Ok(entries) = fs::read_dir(dir) else {
            return Vec::new();
        };

        let mut folders = Vec::new();
        let mut leaf_files = Vec::new();

        for entry in entries.flatten() {
            if files.len() >= max_files {
                break;
            }

            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if path.is_dir() {
                if matches!(name.as_str(), ".git" | "node_modules" | "target" | ".idea") {
                    continue;
                }
                let children = Self::build_tree(base, &path, files, depth + 1, max_files);
                folders.push(TreeNode {
                    abs_path: path,
                    name: name.into(),
                    kind: NodeKind::Folder { children },
                });
                continue;
            }

            let Some(rel_path) = path
                .strip_prefix(base)
                .ok()
                .map(|p| p.to_string_lossy().replace('\\', "/"))
            else {
                continue;
            };
            let (language, icon_name) = language_and_icon_for(&path);
            let file_idx = files.len();

            files.push(FileEntry {
                abs_path: path,
                rel_path: rel_path.into(),
                name: name.clone().into(),
                language,
                icon_name,
            });
            leaf_files.push(TreeNode {
                abs_path: files[file_idx].abs_path.clone(),
                name: name.into(),
                kind: NodeKind::File { file_idx },
            });
        }

        folders.sort_by(|a, b| a.name.to_string().cmp(&b.name.to_string()));
        leaf_files.sort_by(|a, b| a.name.to_string().cmp(&b.name.to_string()));
        folders.extend(leaf_files);
        folders
    }

    fn toggle_folder(&mut self, folder: &Path, cx: &mut Context<Self>) {
        let key = folder.to_path_buf();
        if self.expanded_folders.contains(&key) {
            self.expanded_folders.remove(&key);
        } else {
            self.expanded_folders.insert(key);
        }
        cx.notify();
    }

    fn visible_entries(&self) -> Vec<VisibleEntry> {
        let mut out = Vec::new();
        Self::flatten_visible(&self.tree, 0, &self.expanded_folders, &mut out);
        out
    }

    fn flatten_visible(
        nodes: &[TreeNode],
        depth: usize,
        expanded: &HashSet<PathBuf>,
        out: &mut Vec<VisibleEntry>,
    ) {
        for node in nodes {
            match &node.kind {
                NodeKind::Folder { children } => {
                    let is_expanded = expanded.contains(&node.abs_path);
                    out.push(VisibleEntry {
                        depth,
                        kind: VisibleKind::Folder {
                            abs_path: node.abs_path.clone(),
                            name: node.name.clone(),
                            expanded: is_expanded,
                        },
                    });
                    if is_expanded {
                        Self::flatten_visible(children, depth + 1, expanded, out);
                    }
                }
                NodeKind::File { file_idx } => out.push(VisibleEntry {
                    depth,
                    kind: VisibleKind::File { file_idx: *file_idx },
                }),
            }
        }
    }

    fn open_file_at(&mut self, idx: usize, window: &mut Window, cx: &mut Context<Self>) {
        if idx >= self.files.len() {
            return;
        }

        let path = self.files[idx].abs_path.clone();
        match fs::read_to_string(&path) {
            Ok(content) => {
                self.active_index = Some(idx);
                self.editor_text = content;
                self.cursor_byte = 0;
                self.selection.clear();
                self.hover_byte = None;
                self.editor_hscroll = 0.0;
                if let Some(existing_pos) = self.open_tabs.iter().position(|tab| *tab == idx) {
                    self.open_tabs.remove(existing_pos);
                }
                self.open_tabs.push(idx);
                self.is_dirty = false;
                self.status = format!("Opened {}", self.files[idx].rel_path).into();
                window.focus(&self.editor_focus);
            }
            Err(err) => {
                self.status = format!("Open failed: {}", err).into();
            }
        }

        cx.notify();
    }

    fn save_active_file(&mut self, cx: &mut Context<Self>) {
        let Some(idx) = self.active_index else {
            self.status = "No file selected".into();
            cx.notify();
            return;
        };

        let path = self.files[idx].abs_path.clone();
        match fs::write(&path, &self.editor_text) {
            Ok(_) => {
                self.is_dirty = false;
                self.status = format!("Saved {}", self.files[idx].rel_path).into();
            }
            Err(err) => {
                self.status = format!("Save failed: {}", err).into();
            }
        }

        cx.notify();
    }

    fn clamp_sidebar_width(&mut self) {
        self.sidebar_width = self.sidebar_width.clamp(220.0, 520.0);
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
        if !self.resizing_sidebar || !event.dragging() {
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

    fn explorer_window(&self, total: usize, visible: usize) -> (usize, usize) {
        let visible = visible.max(1);
        if total <= visible {
            return (0, total);
        }
        let max_start = total.saturating_sub(visible);
        let start = self.explorer_scroll.floor().clamp(0.0, max_start as f32) as usize;
        (start, (start + visible).min(total))
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

    fn editor_window(&self, total: usize, visible: usize) -> (usize, usize) {
        let visible = visible.max(1);
        if total <= visible {
            return (0, total);
        }
        let max_start = total.saturating_sub(visible);
        let start = self.editor_scroll.floor().clamp(0.0, max_start as f32) as usize;
        (start, (start + visible).min(total))
    }

    fn editor_h_window(&self, total_cols: usize, visible_cols: usize) -> (usize, usize) {
        let visible_cols = visible_cols.max(1);
        if total_cols <= visible_cols {
            return (0, total_cols);
        }
        let max_start = total_cols.saturating_sub(visible_cols);
        let start = self.editor_hscroll.floor().clamp(0.0, max_start as f32) as usize;
        (start, (start + visible_cols).min(total_cols))
    }

    fn slice_line_by_cols(line: &str, start_col: usize, end_col: usize) -> String {
        line.chars()
            .skip(start_col)
            .take(end_col.saturating_sub(start_col))
            .collect()
    }

    fn editor_text_layout(&self) -> TextLayout {
        TextLayout::from_text(&self.editor_text)
    }

    fn editor_viewport_cells(&self, window: &Window) -> ViewportCells {
        let viewport_w = f32::from(window.viewport_size().width);
        let viewport_h = f32::from(window.viewport_size().height);
        let rows = (((viewport_h - 180.0) / EDITOR_LINE_HEIGHT).floor() as usize).clamp(12, 140);
        let cols = (((viewport_w - self.sidebar_width - 170.0) / EDITOR_GLYPH_WIDTH).floor()
            as usize)
            .clamp(24, 320);
        ViewportCells { rows, cols }
    }

    fn editor_scroll_offset(&self, layout: &TextLayout, viewport: ViewportCells) -> ScrollOffset {
        let max_line_start = layout.line_count().saturating_sub(viewport.rows.max(1));
        let max_col_start = layout.max_line_len().saturating_sub(viewport.cols.max(1));

        ScrollOffset {
            line: self
                .editor_scroll
                .floor()
                .clamp(0.0, max_line_start as f32) as usize,
            column: self
                .editor_hscroll
                .floor()
                .clamp(0.0, max_col_start as f32) as usize,
        }
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
        let layout = self.editor_text_layout();
        let viewport = self.editor_viewport_cells(window);
        let scroll = self.editor_scroll_offset(&layout, viewport);
        let byte = selection::hover_char_index(x, y, self.editor_text_metrics(), scroll, &layout);
        (layout, byte)
    }

    fn begin_selection_drag(
        &mut self,
        event: &MouseDownEvent,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if self.active_index.is_none() || event.button != MouseButton::Left {
            return;
        }
        let x = f32::from(event.position.x);
        let y = f32::from(event.position.y);
        let (layout, byte) = self.hit_test_editor(x, y, window);
        let point = layout.byte_to_point(byte);
        self.cursor_byte = byte;
        self.selection.begin_drag(point);
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
        if self.active_index.is_none() {
            return;
        }
        let x = f32::from(event.position.x);
        let y = f32::from(event.position.y);
        let (layout, byte) = self.hit_test_editor(x, y, window);

        if self.hover_byte != Some(byte) {
            self.hover_byte = Some(byte);
        }

        if self.selection.dragging && event.dragging() {
            self.cursor_byte = byte;
            self.selection.update_drag(layout.byte_to_point(byte));
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
        if event.button != MouseButton::Left || !self.selection.dragging {
            return;
        }
        self.selection.end_drag();
        self.refresh_status();
        cx.notify();
    }

    fn scrollbar_metrics(
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

    fn clamp_cursor_to_boundary(&mut self) {
        if self.cursor_byte > self.editor_text.len() {
            self.cursor_byte = self.editor_text.len();
        }
        while self.cursor_byte > 0 && !self.editor_text.is_char_boundary(self.cursor_byte) {
            self.cursor_byte -= 1;
        }
    }

    fn insert_at_cursor(&mut self, text: &str) {
        self.clamp_cursor_to_boundary();
        self.editor_text.insert_str(self.cursor_byte, text);
        self.cursor_byte += text.len();
        self.is_dirty = true;
    }

    fn delete_backspace(&mut self) {
        self.clamp_cursor_to_boundary();
        if self.cursor_byte == 0 {
            return;
        }
        let prev = self.editor_text[..self.cursor_byte]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.editor_text.replace_range(prev..self.cursor_byte, "");
        self.cursor_byte = prev;
        self.is_dirty = true;
    }

    fn delete_forward(&mut self) {
        self.clamp_cursor_to_boundary();
        if self.cursor_byte >= self.editor_text.len() {
            return;
        }
        let mut iter = self.editor_text[self.cursor_byte..].char_indices();
        let _ = iter.next();
        let next = iter
            .next()
            .map(|(i, _)| self.cursor_byte + i)
            .unwrap_or(self.editor_text.len());
        self.editor_text.replace_range(self.cursor_byte..next, "");
        self.is_dirty = true;
    }

    fn move_left(&mut self) {
        self.clamp_cursor_to_boundary();
        if self.cursor_byte == 0 {
            return;
        }
        self.cursor_byte = self.editor_text[..self.cursor_byte]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
    }

    fn move_right(&mut self) {
        self.clamp_cursor_to_boundary();
        if self.cursor_byte >= self.editor_text.len() {
            return;
        }
        let mut iter = self.editor_text[self.cursor_byte..].char_indices();
        let _ = iter.next();
        self.cursor_byte = iter
            .next()
            .map(|(i, _)| self.cursor_byte + i)
            .unwrap_or(self.editor_text.len());
    }

    fn line_col_from_byte(&self, byte: usize) -> (usize, usize) {
        let mut line = 0usize;
        let mut col = 0usize;
        for (idx, ch) in self.editor_text.char_indices() {
            if idx >= byte {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    fn byte_from_line_col(&self, target_line: usize, target_col: usize) -> usize {
        let mut line = 0usize;
        let mut col = 0usize;
        for (idx, ch) in self.editor_text.char_indices() {
            if line == target_line && col == target_col {
                return idx;
            }
            if ch == '\n' {
                if line == target_line {
                    return idx;
                }
                line += 1;
                col = 0;
            } else if line == target_line {
                col += 1;
            }
        }
        self.editor_text.len()
    }

    fn move_up(&mut self) {
        self.clamp_cursor_to_boundary();
        let (line, col) = self.line_col_from_byte(self.cursor_byte);
        if line == 0 {
            return;
        }
        self.cursor_byte = self.byte_from_line_col(line - 1, col);
    }

    fn move_down(&mut self) {
        self.clamp_cursor_to_boundary();
        let (line, col) = self.line_col_from_byte(self.cursor_byte);
        self.cursor_byte = self.byte_from_line_col(line + 1, col);
    }

    fn move_home(&mut self) {
        self.clamp_cursor_to_boundary();
        let before = &self.editor_text[..self.cursor_byte];
        self.cursor_byte = before.rfind('\n').map_or(0, |i| i + 1);
    }

    fn move_end(&mut self) {
        self.clamp_cursor_to_boundary();
        let after = &self.editor_text[self.cursor_byte..];
        self.cursor_byte = after
            .find('\n')
            .map(|i| self.cursor_byte + i)
            .unwrap_or(self.editor_text.len());
    }

    fn refresh_status(&mut self) {
        let (line, col) = self.line_col_from_byte(self.cursor_byte);
        self.status = if self.is_dirty {
            format!("Modified (Ctrl+S to save) | Ln {}, Col {}", line + 1, col + 1).into()
        } else {
            format!("Ln {}, Col {}", line + 1, col + 1).into()
        };
    }

    fn handle_editor_key(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.active_index.is_none() {
            return;
        }

        let mods = event.keystroke.modifiers;
        let cmd_or_ctrl = mods.control || mods.platform;

        if cmd_or_ctrl && event.keystroke.key.eq_ignore_ascii_case("s") {
            self.save_active_file(cx);
            return;
        }

        if cmd_or_ctrl || mods.alt || mods.function {
            return;
        }

        match event.keystroke.key.as_str() {
            "left" => self.move_left(),
            "right" => self.move_right(),
            "up" => self.move_up(),
            "down" => self.move_down(),
            "home" => self.move_home(),
            "end" => self.move_end(),
            "backspace" => {
                self.delete_backspace();
            }
            "delete" => {
                self.delete_forward();
            }
            "enter" => {
                self.insert_at_cursor("\n");
            }
            "tab" => {
                self.insert_at_cursor("    ");
            }
            _ => {
                if let Some(ch) = &event.keystroke.key_char {
                    self.insert_at_cursor(ch);
                }
            }
        }

        self.selection.clear();
        self.hover_byte = None;
        self.refresh_status();
        cx.notify();
    }

    fn render_welcome(&mut self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .size_full()
            .bg(rgb(0x020202))
            .text_color(rgb(0xF2F5FB))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(700.0))
                    .rounded_lg()
                    .bg(rgb(0x1D2230))
                    .p_6()
                    .flex_col()
                    .gap_3()
                    .child("Velo")
                    .child("A Rust + GPUI code editor")
                    .child("Start like VS Code: open a folder and begin editing.")
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(rgb(0x222A3A))
                            .id("welcome-open-folder")
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.open_project_dialog(window, cx);
                            }))
                            .child(img(self.icons.folder_open()).size(px(16.0)))
                            .child("Open Folder"),
                    )
                    .child("Tips: Click a file in Explorer, type in editor, press Ctrl+S to save."),
            )
            .into_any_element()
    }

    fn render_workspace(&mut self, cx: &mut Context<Self>, window: &mut Window) -> AnyElement {
        let viewport_w = f32::from(window.viewport_size().width);
        let viewport_h = f32::from(window.viewport_size().height);
        let explorer_visible_rows = (((viewport_h - 250.0) / 22.0).floor() as usize).clamp(8, 80);
        let entries = self.visible_entries();
        let max_scroll = entries.len().saturating_sub(explorer_visible_rows) as f32;
        self.explorer_scroll = self.explorer_scroll.clamp(0.0, max_scroll);

        let project_name: SharedString = self
            .project_root
            .as_ref()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string().into()))
            .unwrap_or_else(|| "Workspace".into());

        let active_language = self
            .active_index
            .map(|idx| self.files[idx].language)
            .unwrap_or("text");
        let display_text = self.editor_text.clone();
        let editor_lines: Vec<&str> = display_text.split('\n').collect();
        let editor_total = editor_lines.len().max(1);
        let max_line_cols = editor_lines
            .iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(1);
        let editor_visible_rows =
            (((viewport_h - 180.0) / EDITOR_LINE_HEIGHT).floor() as usize).clamp(12, 140);
        let editor_visible_cols =
            (((viewport_w - self.sidebar_width - 170.0) / EDITOR_GLYPH_WIDTH).floor() as usize)
                .clamp(24, 320);
        let editor_max_scroll = editor_total.saturating_sub(editor_visible_rows) as f32;
        let editor_hmax_scroll = max_line_cols.saturating_sub(editor_visible_cols) as f32;
        self.editor_scroll = self.editor_scroll.clamp(0.0, editor_max_scroll);
        self.editor_hscroll = self.editor_hscroll.clamp(0.0, editor_hmax_scroll);
        let (editor_start, editor_end) = self.editor_window(editor_total, editor_visible_rows);
        let (editor_col_start, editor_col_end) =
            self.editor_h_window(max_line_cols.max(1), editor_visible_cols);
        let viewport_lines = editor_lines[editor_start..editor_end]
            .iter()
            .map(|line| Self::slice_line_by_cols(line, editor_col_start, editor_col_end))
            .collect::<Vec<_>>();
        let editor_viewport = viewport_lines.join("\n");
        let line_number_width = editor_total.to_string().len().max(2);
        let line_numbers = (editor_start..editor_end)
            .map(|line| format!("{:>width$}", line + 1, width = line_number_width))
            .collect::<Vec<_>>()
            .join("\n");
        let text_layout = TextLayout::from_text(&self.editor_text);
        let selection_rects = selection::selection_rects(
            &self.selection,
            &text_layout,
            self.editor_text_metrics(),
            ScrollOffset {
                line: editor_start,
                column: editor_col_start,
            },
            ViewportCells {
                rows: editor_visible_rows,
                cols: editor_visible_cols,
            },
        );
        let highlighted = syntax_highlighted_text(&editor_viewport, active_language);
        let editor_track_h = (editor_visible_rows as f32 * 17.5).clamp(120.0, 820.0);
        let (editor_thumb_h, editor_thumb_top, editor_scrollable) = Self::scrollbar_metrics(
            editor_total,
            editor_visible_rows,
            editor_start,
            editor_track_h,
        );
        let editor_htrack_w = (editor_visible_cols as f32 * 8.2).clamp(120.0, 1400.0);
        let (editor_hthumb_w, editor_hthumb_left, editor_hscrollable) = Self::scrollbar_metrics(
            max_line_cols.max(1),
            editor_visible_cols,
            editor_col_start,
            editor_htrack_w,
        );
        let (start, end) = self.explorer_window(entries.len(), explorer_visible_rows);
        let visible_entries = &entries[start..end];
        let tab_start = self.open_tabs.len().saturating_sub(8);
        let visible_tabs = &self.open_tabs[tab_start..];
        let total = entries.len().max(1);
        let track_h = (explorer_visible_rows as f32 * 22.0).clamp(150.0, 760.0);
        let (thumb_h, thumb_top, explorer_scrollable) =
            Self::scrollbar_metrics(total, explorer_visible_rows, start, track_h);

        div()
            .size_full()
            .bg(rgb(0x020202))
            .text_color(rgb(0xF2F5FB))
            .child(
                div()
                    .size_full()
                    .flex_col()
                    .child(
                        div()
                            .h(px(28.0))
                            .px_2()
                            .flex()
                            .items_center()
                            .gap_3()
                            .bg(rgb(0x171B24))
                            .text_color(rgb(0xB6BDCB))
                            .child("File")
                            .child("Edit")
                            .child("Selection")
                            .child("View")
                            .child("Go")
                            .child("Run")
                            .child("Terminal")
                            .child("Help"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                    .child(
                        div()
                            .w(px(72.0))
                            .h_full()
                            .bg(rgb(0x171B24))
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
                                            .bg(rgb(0x23324A))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(img(self.icons.folder_open()).size(px(24.0))),
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .h(px(56.0))
                                            .bg(rgb(0x1D2230))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(img(self.icons.file()).size(px(24.0))),
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .h(px(56.0))
                                            .bg(rgb(0x1D2230))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(img(self.icons.run()).size(px(24.0))),
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
                                            .bg(rgb(0x1D2230))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(img(self.icons.settings()).size(px(24.0))),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .w(px((self.sidebar_width - 58.0).max(220.0)))
                            .h_full()
                            .p_2()
                            .overflow_hidden()
                            .bg(rgb(0x1D2230))
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
                                    .bg(rgb(0x171B24))
                                    .text_color(rgb(0x4FAEFF))
                                    .child("Explorer"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                                        this.scroll_explorer(event, cx);
                                    }))
                                    .child(
                                        div().flex_1().flex_col().gap_1().children(
                                    visible_entries.iter().enumerate().map(|(visible_idx, row)| {
                                        let row_id = start + visible_idx;
                                        match &row.kind {
                                            VisibleKind::Folder { abs_path, name, expanded } => {
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
                                                    .bg(rgb(0x171B24))
                                                    .id(("explorer-folder", row_id))
                                                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                                        this.toggle_folder(&folder, cx);
                                                    }))
                                                    .child(div().w(px((row.depth as f32) * 14.0)))
                                                    .child(img(icon).size(px(17.0)))
                                                    .child(div().flex_1().truncate().child(name.clone()))
                                            }
                                            VisibleKind::File { file_idx } => {
                                                let idx = *file_idx;
                                                let file = &self.files[idx];
                                                let selected = self.active_index == Some(idx);
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .overflow_hidden()
                                                    .gap_2()
                                                    .px_2()
                                                    .py(px(2.0))
                                                    .rounded_sm()
                                                    .bg(if selected { rgb(0x23324A) } else { rgb(0x171B24) })
                                                    .id(("explorer-file", row_id))
                                                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                                        this.open_file_at(idx, window, cx);
                                                    }))
                                                    .child(div().w(px((row.depth as f32) * 14.0)))
                                                    .child(img(this_icon(&self.icons, file)).size(px(17.0)))
                                                    .child(div().flex_1().truncate().child(file.name.clone()))
                                            }
                                        }
                                    }),
                                ),
                                    )
                                    .child(
                                        div()
                                            .w(px(8.0))
                                            .h(px(track_h))
                                            .rounded_md()
                                            .bg(rgb(0x222A3A))
                                            .child(
                                                div()
                                                    .w(px(8.0))
                                                    .h(px(thumb_h))
                                                    .mt(px(thumb_top))
                                                    .rounded_md()
                                                    .bg(if explorer_scrollable {
                                                        rgb(0x8F98AA)
                                                    } else {
                                                        rgb(0x222A3A)
                                                    }),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .w(px(6.0))
                            .h_full()
                            .bg(rgb(0x222A3A))
                            .id("sidebar-splitter")
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
                                    .overflow_hidden()
                                    .children(
                                    visible_tabs.iter().map(|tab_idx| {
                                        let file = &self.files[*tab_idx];
                                        let selected = self.active_index == Some(*tab_idx);
                                        let mut label = Self::compact_label(&file.rel_path, 22).to_string();
                                        if selected && self.is_dirty {
                                            label.push_str(" *");
                                        }
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .overflow_hidden()
                                            .px_3()
                                            .py_1()
                                            .w(px(150.0))
                                            .rounded_md()
                                            .bg(if selected { rgb(0x23324A) } else { rgb(0x171B24) })
                                            .id(("tab", *tab_idx))
                                            .on_click(cx.listener({
                                                let tab_idx = *tab_idx;
                                                move |this, _: &ClickEvent, window, cx| {
                                                    this.open_file_at(tab_idx, window, cx);
                                                }
                                            }))
                                            .child(img(this_icon(&self.icons, file)).size(px(13.0)))
                                            .child(div().flex_1().truncate().child(label))
                                    }),
                                ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .rounded_md()
                                    .bg(rgb(0x171B24))
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
                                                    .bg(rgb(0x1D2230))
                                                    .p_2()
                                                    .overflow_hidden()
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
                                                            .size_full()
                                                            .relative()
                                                            .flex()
                                                            .gap_2()
                                                            .child(
                                                                div()
                                                                    .w(px(52.0))
                                                                    .h_full()
                                                                    .bg(rgb(0x171B24))
                                                                    .text_color(rgb(0x69748A))
                                                                    .text_right()
                                                                    .px_2()
                                                                    .child(line_numbers),
                                                            )
                                                            .child(
                                                                div()
                                                                    .flex_1()
                                                                    .h_full()
                                                                    .overflow_hidden()
                                                                    .relative()
                                                                    .children(selection_rects.iter().enumerate().map(|(idx, rect)| {
                                                                        div()
                                                                            .id(("selection-rect", idx))
                                                                            .absolute()
                                                                            .left(px(rect.x))
                                                                            .top(px(rect.y))
                                                                            .w(px(rect.width))
                                                                            .h(px(rect.height))
                                                                            .bg(rgb(0x2B3E62))
                                                                    }))
                                                                    .child(
                                                                        div()
                                                                            .absolute()
                                                                            .top_0()
                                                                            .left_0()
                                                                            .size_full()
                                                                            .child(highlighted),
                                                                    ),
                                                            ),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .w(px(10.0))
                                                    .h(px(editor_track_h))
                                                    .rounded_md()
                                                    .bg(rgb(0x222A3A))
                                                    .child(
                                                        div()
                                                            .w(px(10.0))
                                                            .h(px(editor_thumb_h))
                                                            .mt(px(editor_thumb_top))
                                                            .rounded_md()
                                                            .bg(if editor_scrollable {
                                                                rgb(0xB6BDCB)
                                                            } else {
                                                                rgb(0x222A3A)
                                                            }),
                                                    ),
                                            )
                                    )
                                    .child(
                                        div()
                                            .h(px(10.0))
                                            .w_full()
                                            .rounded_md()
                                            .bg(rgb(0x222A3A))
                                            .child(
                                                div()
                                                    .h(px(10.0))
                                                    .w(px(editor_hthumb_w))
                                                    .ml(px(editor_hthumb_left))
                                                    .rounded_md()
                                                    .bg(if editor_hscrollable {
                                                        rgb(0xB6BDCB)
                                                    } else {
                                                        rgb(0x222A3A)
                                                    }),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .h(px(24.0))
                                    .rounded_md()
                                    .px_2()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .bg(rgb(0x171B24))
                                    .text_color(rgb(0x8F98AA))
                                    .child(self.status.clone())
                                    .child(format!("{} | GPUI", active_language)),
                            ),
                    ),
                    ),
            )
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
                if this.active_index.is_some() {
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





