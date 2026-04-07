use crate::ui::{core::EditorCore, file_text::decode_text_file, workspace::WorkspaceState};
use std::fs;

pub enum OpenFileResult {
    InvalidIndex,
    OpenFailed(String),
    Opened { status: String },
}

pub enum SaveResult {
    NoFileSelected,
    SaveFailed(String),
    Saved { status: String },
}

pub fn open_file_into_editor(
    workspace: &mut WorkspaceState,
    core: &mut EditorCore,
    idx: usize,
) -> OpenFileResult {
    if idx >= workspace.files.len() {
        return OpenFileResult::InvalidIndex;
    }

    let path = workspace.files[idx].abs_path.clone();
    match decode_text_file(&path) {
        Ok(decoded) => {
            workspace.open_file_tab(idx);
            // Normalize all EOL variants to '\n' so layout, hit-testing, and selection
            // geometry stay consistent across files originally saved with CRLF/CR.
            let mut normalized = decoded.text.replace("\r\n", "\n").replace('\r', "\n");
            if normalized.is_empty() {
                if let Ok(raw_text) = fs::read_to_string(&path) {
                    let fallback = raw_text.replace("\r\n", "\n").replace('\r', "\n");
                    if !fallback.is_empty() {
                        normalized = fallback;
                    }
                }
            }
            let char_count = normalized.chars().count();
            core.set_text(normalized);
            let status = if decoded.had_errors {
                format!(
                    "Opened {} [{}{}] | {} chars (lossy decode)",
                    workspace.files[idx].rel_path,
                    decoded.encoding.name(),
                    if decoded.has_bom { ", BOM" } else { "" },
                    char_count
                )
            } else {
                format!(
                    "Opened {} [{}{}] | {} chars",
                    workspace.files[idx].rel_path,
                    decoded.encoding.name(),
                    if decoded.has_bom { ", BOM" } else { "" },
                    char_count
                )
            };
            OpenFileResult::Opened { status }
        }
        Err(err) => OpenFileResult::OpenFailed(err.to_string()),
    }
}

pub fn save_active_file(workspace: &WorkspaceState, core: &mut EditorCore) -> SaveResult {
    let Some(idx) = workspace.active_index else {
        return SaveResult::NoFileSelected;
    };

    let path = workspace.files[idx].abs_path.clone();
    match fs::write(&path, &core.text) {
        Ok(_) => {
            core.mark_saved();
            SaveResult::Saved {
                status: format!("Saved {}", workspace.files[idx].rel_path),
            }
        }
        Err(err) => SaveResult::SaveFailed(err.to_string()),
    }
}
