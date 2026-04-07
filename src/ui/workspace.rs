use crate::ui::language::language_and_icon_for;
use gpui::SharedString;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct FileEntry {
    pub abs_path: PathBuf,
    pub rel_path: SharedString,
    pub name: SharedString,
    pub language: &'static str,
    pub icon_name: &'static str,
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
pub enum VisibleKind {
    Folder {
        abs_path: PathBuf,
        name: SharedString,
        expanded: bool,
    },
    File {
        file_idx: usize,
    },
}

#[derive(Clone)]
pub struct VisibleEntry {
    pub depth: usize,
    pub kind: VisibleKind,
}

#[derive(Default)]
pub struct WorkspaceState {
    pub project_root: Option<PathBuf>,
    pub files: Vec<FileEntry>,
    tree: Vec<TreeNode>,
    expanded_folders: HashSet<PathBuf>,
    pub open_tabs: Vec<usize>,
    pub active_index: Option<usize>,
}

impl WorkspaceState {
    pub fn load_project_index(&mut self, root: PathBuf, max_files: usize) {
        let mut files = Vec::new();
        let tree = Self::build_tree(&root, &root, &mut files, 0, max_files);
        self.project_root = Some(root.clone());
        self.files = files;
        self.tree = tree;
        self.expanded_folders.clear();
        self.expanded_folders.insert(root);
        self.open_tabs.clear();
        self.active_index = None;
    }

    pub fn toggle_folder(&mut self, folder: &Path) {
        let key = folder.to_path_buf();
        if self.expanded_folders.contains(&key) {
            self.expanded_folders.remove(&key);
        } else {
            self.expanded_folders.insert(key);
        }
    }

    pub fn visible_entries(&self) -> Vec<VisibleEntry> {
        let mut out = Vec::new();
        Self::flatten_visible(&self.tree, 0, &self.expanded_folders, &mut out);
        out
    }

    pub fn open_file_tab(&mut self, idx: usize) {
        self.active_index = Some(idx);
        if let Some(existing_pos) = self.open_tabs.iter().position(|tab| *tab == idx) {
            self.open_tabs.remove(existing_pos);
        }
        self.open_tabs.push(idx);
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
                    kind: VisibleKind::File {
                        file_idx: *file_idx,
                    },
                }),
            }
        }
    }
}
