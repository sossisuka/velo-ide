use std::path::Path;

pub fn language_and_icon_for(path: &Path) -> (&'static str, &'static str) {
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    if file_name == ".env" || file_name.starts_with(".env.") {
        return ("dotenv", "settings");
    }

    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "rs" => ("rust", "rust"),
        "ts" | "tsx" => ("typescript", "typescript"),
        "js" | "jsx" | "mjs" | "cjs" => ("javascript", "js"),
        "py" => ("python", "python"),
        "go" => ("go", "go"),
        "java" => ("java", "java"),
        "c" | "h" => ("c", "c"),
        "cpp" | "cc" | "cxx" | "hpp" | "hh" => ("cpp", "cpp"),
        "cs" => ("csharp", "csharp"),
        "php" => ("php", "php"),
        "rb" => ("ruby", "ruby"),
        "kt" | "kts" => ("kotlin", "kotlin"),
        "swift" => ("swift", "swift"),
        "html" | "htm" => ("html", "html"),
        "css" | "scss" => ("css", "css"),
        "json" => ("json", "json"),
        "yaml" | "yml" => ("yaml", "yaml"),
        "toml" => ("toml", "toml"),
        "md" | "markdown" => ("markdown", "markdown"),
        "sh" => ("shell", "shell"),
        "ps1" => ("powershell", "powershell"),
        "sql" => ("sql", "sql"),
        "xml" => ("xml", "xml"),
        "vue" => ("vue", "vue"),
        "svelte" => ("svelte", "svelte"),
        "astro" => ("astro", "astro"),
        _ => ("text", "file"),
    }
}
