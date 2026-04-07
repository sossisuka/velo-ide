use std::ops::Range;

use gpui::{rgb, HighlightStyle, StyledText};

pub fn syntax_highlighted_text(text: &str, language: &str) -> StyledText {
    let keyword_color = HighlightStyle {
        color: Some(rgb(0x4FAEFF).into()),
        ..Default::default()
    };
    let type_color = HighlightStyle {
        color: Some(rgb(0xB6BDCB).into()),
        ..Default::default()
    };
    let comment_color = HighlightStyle {
        color: Some(rgb(0x8F98AA).into()),
        ..Default::default()
    };

    let mut ranges: Vec<(Range<usize>, HighlightStyle)> = Vec::new();

    let keywords: &[&str] = match language {
        "rust" => &[
            "fn", "let", "mut", "pub", "impl", "struct", "enum", "trait", "match", "if", "else",
            "for", "while", "loop", "use", "mod", "return", "async", "await",
        ],
        "typescript" | "javascript" => &[
            "function", "const", "let", "var", "class", "interface", "type", "if", "else", "for",
            "while", "return", "import", "export", "async", "await", "new",
        ],
        "python" => &[
            "def", "class", "import", "from", "if", "elif", "else", "for", "while", "return", "try",
            "except", "with", "as", "lambda", "async", "await",
        ],
        "go" => &[
            "func", "package", "import", "var", "const", "type", "struct", "interface", "if", "else",
            "for", "range", "return", "go", "defer",
        ],
        "java" | "c" | "cpp" | "csharp" | "php" | "kotlin" | "swift" => &[
            "class", "public", "private", "protected", "static", "void", "int", "string", "if", "else",
            "for", "while", "return", "new", "import", "package",
        ],
        "ruby" => &[
            "def", "class", "module", "if", "elsif", "else", "do", "end", "require", "return",
        ],
        "json" => &["true", "false", "null"],
        "yaml" | "toml" => &["true", "false"],
        "sql" => &[
            "select", "from", "where", "insert", "update", "delete", "join", "inner", "left", "right",
            "group", "order", "by", "limit", "as",
        ],
        "html" | "xml" => &["<", "</", "/>"],
        "css" => &["@media", "@keyframes", "display", "position", "color", "background"],
        _ => &[],
    };

    for kw in keywords {
        ranges.extend(find_token_ranges(text, kw).into_iter().map(|r| (r, keyword_color)));
    }

    for kw in ["String", "Result", "Option", "Vec", "Self"] {
        ranges.extend(find_token_ranges(text, kw).into_iter().map(|r| (r, type_color)));
    }

    let comment_prefixes: &[&str] = match language {
        "python" | "yaml" | "toml" | "shell" => &["#"],
        "sql" => &["--"],
        _ => &["//"],
    };

    let mut offset = 0usize;
    for line in text.split_inclusive('\n') {
        for prefix in comment_prefixes {
            if let Some(at) = line.find(prefix) {
                ranges.push((offset + at..offset + line.len(), comment_color));
                break;
            }
        }
        offset += line.len();
    }

    ranges = sanitize_highlight_ranges(text, ranges);

    StyledText::new(text.to_owned()).with_highlights(ranges)
}

fn find_token_ranges(text: &str, token: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    if token.is_empty() {
        return out;
    }

    let mut start = 0usize;
    while let Some(idx) = text[start..].find(token) {
        let real_start = start + idx;
        let real_end = real_start + token.len();

        let left_ok = real_start == 0
            || !text[..real_start]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
        let right_ok = real_end == text.len()
            || !text[real_end..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');

        if left_ok && right_ok {
            out.push(real_start..real_end);
        }

        start = real_end;
    }

    out
}

fn sanitize_highlight_ranges(
    text: &str,
    mut ranges: Vec<(Range<usize>, HighlightStyle)>,
) -> Vec<(Range<usize>, HighlightStyle)> {
    ranges.retain(|(r, _)| {
        r.start < r.end
            && r.end <= text.len()
            && text.is_char_boundary(r.start)
            && text.is_char_boundary(r.end)
    });
    ranges.sort_by_key(|(r, _)| r.start);

    let mut out: Vec<(Range<usize>, HighlightStyle)> = Vec::with_capacity(ranges.len());
    let mut last_end = 0usize;
    for (r, style) in ranges {
        if r.start < last_end {
            continue;
        }
        last_end = r.end;
        out.push((r, style));
    }
    out
}
