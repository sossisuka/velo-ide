# Velo IDE (Rust + GPUI)

`Velo` is a starter IDE-like desktop UI written in Rust using `gpui`.

## What is included

- VS Code source cloned to: `git tmp/vscode`
- Rust project: `velo`
- VS Code-like start screen (`Open Folder`)
- Explorer with file icons mapped by language extension
- Basic file open + inline editing + save (`Ctrl+S`)
- Cursor-based editing (arrows, Home/End, Backspace/Delete, Enter/Tab)
- Open-file tabs (VS Code style top tab strip)
- Top menu bar (`File Edit Selection View Go Run Terminal Help`)
- Resizable left sidebar (drag the splitter)
- Explorer custom styled scrollbar
- Tree Explorer with expandable folders (click folder to toggle contents)
- Starter syntax highlighting (keywords/comments) for common languages
- Bearded Icons assets copied to `assets/icons/bearded`

## UI code structure

- `src/main.rs` — app bootstrap and window creation
- `src/ui/app.rs` — main UI state + rendering + interactions
- `src/ui/language.rs` — file-extension to language/icon mapping
- `src/ui/highlight.rs` — syntax highlight helpers
- `src/ui/mod.rs` — module exports

## Run

```powershell
cd velo
cargo run
```

If `cargo` is not installed, install Rust from <https://rustup.rs> first.
