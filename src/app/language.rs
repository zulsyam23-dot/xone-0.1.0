//! Module: src/app/language.rs
//! Catatan: file ini bagian dari mesin Xone; ubah logika dengan hati-hati, kopi tetap pahit.

use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Language {
    PlainText,
    Html,
    Css,
    JavaScript,
    TypeScript,
    Rust,
    Json,
    Markdown,
    Python,
    C,
    Cpp,
    CSharp,
    Java,
    Go,
    Yaml,
    Toml,
    Shell,
    PowerShell,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Plain,
    Keyword,
    Type,
    String,
    Number,
    Comment,
    Tag,
    Attribute,
    Value,
    Operator,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub text: String,
    pub kind: TokenKind,
}

impl Token {
    fn new(text: String, kind: TokenKind) -> Self {
        Self { text, kind }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Suggestion {
    pub label: &'static str,
    pub insert: &'static str,
}

pub fn detect_language(path: &Path) -> Language {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());

    match ext.as_deref() {
        Some("html") | Some("htm") => Language::Html,
        Some("css") => Language::Css,
        Some("js") | Some("mjs") | Some("cjs") => Language::JavaScript,
        Some("ts") | Some("tsx") => Language::TypeScript,
        Some("rs") => Language::Rust,
        Some("json") => Language::Json,
        Some("md") | Some("markdown") => Language::Markdown,
        Some("py") => Language::Python,
        Some("c") => Language::C,
        Some("cc") | Some("cpp") | Some("cxx") | Some("hpp") | Some("hh") | Some("hxx") => {
            Language::Cpp
        }
        Some("cs") => Language::CSharp,
        Some("java") => Language::Java,
        Some("go") => Language::Go,
        Some("yaml") | Some("yml") => Language::Yaml,
        Some("toml") => Language::Toml,
        Some("sh") | Some("bash") | Some("zsh") => Language::Shell,
        Some("ps1") | Some("psm1") => Language::PowerShell,
        _ => Language::PlainText,
    }
}

pub fn language_label(language: Language) -> &'static str {
    match language {
        Language::PlainText => "Plain Text",
        Language::Html => "HTML",
        Language::Css => "CSS",
        Language::JavaScript => "JavaScript",
        Language::TypeScript => "TypeScript",
        Language::Rust => "Rust",
        Language::Json => "JSON",
        Language::Markdown => "Markdown",
        Language::Python => "Python",
        Language::C => "C",
        Language::Cpp => "C++",
        Language::CSharp => "C#",
        Language::Java => "Java",
        Language::Go => "Go",
        Language::Yaml => "YAML",
        Language::Toml => "TOML",
        Language::Shell => "Shell",
        Language::PowerShell => "PowerShell",
    }
}

pub fn highlight_line(language: Language, line: &str) -> Vec<Token> {
    match language {
        Language::Html => tokenize_html_line(line),
        Language::Css => tokenize_code_line(
            line,
            &[
                "display",
                "position",
                "flex",
                "grid",
                "align-items",
                "justify-content",
                "color",
                "background",
                "margin",
                "padding",
                "border",
                "font-size",
                "font-weight",
                "width",
                "height",
            ],
            &["/*", "//"],
            &["px", "rem", "em", "vh", "vw"],
        ),
        Language::JavaScript => tokenize_code_line(
            line,
            &[
                "const", "let", "var", "function", "return", "if", "else", "for", "while",
                "switch", "case", "break", "continue", "import", "export", "from", "class", "new",
                "async", "await", "try", "catch", "finally", "throw",
            ],
            &["//"],
            &["string", "number", "boolean", "object", "array"],
        ),
        Language::TypeScript => tokenize_code_line(
            line,
            &[
                "const",
                "let",
                "var",
                "function",
                "return",
                "if",
                "else",
                "for",
                "while",
                "switch",
                "case",
                "break",
                "continue",
                "import",
                "export",
                "from",
                "class",
                "new",
                "async",
                "await",
                "try",
                "catch",
                "finally",
                "throw",
                "interface",
                "type",
                "extends",
                "implements",
            ],
            &["//"],
            &[
                "string", "number", "boolean", "void", "unknown", "any", "never",
            ],
        ),
        Language::Rust => tokenize_code_line(
            line,
            &[
                "fn", "let", "mut", "if", "else", "for", "while", "loop", "match", "return",
                "struct", "enum", "impl", "trait", "mod", "use", "pub", "crate", "self", "super",
                "where", "as", "const", "static", "async", "await", "move",
            ],
            &["//"],
            &[
                "String", "Vec", "Option", "Result", "i32", "i64", "u32", "u64", "usize", "isize",
                "bool", "char", "str",
            ],
        ),
        Language::Json => tokenize_code_line(line, &["true", "false", "null"], &[], &[]),
        Language::Markdown => tokenize_code_line(line, &["#", "##", "###", "-", "*"], &[], &[]),
        Language::Python => tokenize_code_line(
            line,
            &[
                "def", "class", "if", "elif", "else", "for", "while", "return", "import", "from",
                "as", "try", "except", "finally", "with", "lambda", "pass", "break", "continue",
                "yield",
            ],
            &["#"],
            &[
                "str", "int", "float", "bool", "dict", "list", "tuple", "set",
            ],
        ),
        Language::C => tokenize_code_line(
            line,
            &[
                "int", "char", "float", "double", "void", "if", "else", "for", "while", "switch",
                "case", "break", "continue", "return", "struct", "typedef", "enum", "static",
                "const", "sizeof",
            ],
            &["//"],
            &["size_t", "uint32_t", "uint64_t", "bool"],
        ),
        Language::Cpp => tokenize_code_line(
            line,
            &[
                "int",
                "char",
                "float",
                "double",
                "void",
                "if",
                "else",
                "for",
                "while",
                "switch",
                "case",
                "break",
                "continue",
                "return",
                "struct",
                "class",
                "namespace",
                "using",
                "template",
                "typename",
                "const",
                "static",
                "auto",
                "public",
                "private",
                "protected",
            ],
            &["//"],
            &[
                "std",
                "string",
                "vector",
                "map",
                "unordered_map",
                "size_t",
                "bool",
                "nullptr",
            ],
        ),
        Language::CSharp => tokenize_code_line(
            line,
            &[
                "using",
                "namespace",
                "class",
                "interface",
                "public",
                "private",
                "protected",
                "internal",
                "static",
                "void",
                "return",
                "if",
                "else",
                "for",
                "foreach",
                "while",
                "switch",
                "case",
                "break",
                "continue",
                "new",
            ],
            &["//"],
            &[
                "string", "int", "long", "bool", "double", "decimal", "object", "var",
            ],
        ),
        Language::Java => tokenize_code_line(
            line,
            &[
                "package",
                "import",
                "class",
                "interface",
                "public",
                "private",
                "protected",
                "static",
                "void",
                "return",
                "if",
                "else",
                "for",
                "while",
                "switch",
                "case",
                "break",
                "continue",
                "new",
                "extends",
                "implements",
            ],
            &["//"],
            &[
                "String", "Integer", "Long", "Boolean", "Double", "Object", "List", "Map",
            ],
        ),
        Language::Go => tokenize_code_line(
            line,
            &[
                "package",
                "import",
                "func",
                "return",
                "if",
                "else",
                "for",
                "switch",
                "case",
                "break",
                "continue",
                "go",
                "defer",
                "struct",
                "interface",
                "type",
                "var",
                "const",
            ],
            &["//"],
            &[
                "string", "int", "int64", "uint", "bool", "error", "byte", "rune", "map", "chan",
            ],
        ),
        Language::Yaml => tokenize_code_line(line, &["true", "false", "null"], &["#"], &[]),
        Language::Toml => tokenize_code_line(line, &["true", "false"], &["#"], &[]),
        Language::Shell => tokenize_code_line(
            line,
            &[
                "if", "then", "else", "fi", "for", "in", "do", "done", "case", "esac", "function",
                "return",
            ],
            &["#"],
            &["echo", "grep", "sed", "awk", "cat", "ls", "cd", "pwd"],
        ),
        Language::PowerShell => tokenize_code_line(
            line,
            &[
                "function", "param", "if", "else", "foreach", "for", "while", "switch", "return",
                "try", "catch", "finally",
            ],
            &["#"],
            &[
                "string",
                "int",
                "bool",
                "object",
                "hashtable",
                "array",
                "pscustomobject",
            ],
        ),
        Language::PlainText => vec![Token::new(line.to_string(), TokenKind::Plain)],
    }
}

pub fn suggestions_for_line(language: Language, before_cursor: &str) -> Vec<Suggestion> {
    let trimmed = before_cursor.trim_start();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    match language {
        Language::Html => {
            if trimmed.ends_with("<!") {
                out.push(Suggestion {
                    label: "HTML doctype + root",
                    insert: "DOCTYPE html>\n<html lang=\"en\">\n<head>\n    <meta charset=\"UTF-8\" />\n    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" />\n    <title>Document</title>\n</head>\n<body>\n    \n</body>\n</html>",
                });
            }
            if trimmed.ends_with('<') {
                out.push(Suggestion {
                    label: "HTML div container",
                    insert: "div class=\"container\"></div>",
                });
                out.push(Suggestion {
                    label: "HTML section block",
                    insert: "section class=\"section\">\n    \n</section>",
                });
                out.push(Suggestion {
                    label: "HTML script tag",
                    insert: "script src=\"app.js\"></script>",
                });
            }
        }
        Language::Css => {
            if trimmed.ends_with('{') {
                out.push(Suggestion {
                    label: "CSS flex block",
                    insert: "\n    display: flex;\n    align-items: center;\n    justify-content: center;\n}",
                });
                out.push(Suggestion {
                    label: "CSS layout block",
                    insert: "\n    width: 100%;\n    max-width: 1200px;\n    margin: 0 auto;\n}",
                });
            }
        }
        Language::Rust => {
            if trimmed.ends_with("fn ") {
                out.push(Suggestion {
                    label: "Rust function skeleton",
                    insert: "name() {\n    \n}",
                });
                out.push(Suggestion {
                    label: "Rust function -> Result",
                    insert: "name() -> Result<(), Box<dyn std::error::Error>> {\n    Ok(())\n}",
                });
            }
            if trimmed.ends_with("if ") {
                out.push(Suggestion {
                    label: "Rust if block",
                    insert: "condition {\n    \n}",
                });
            }
            if trimmed.ends_with("for ") {
                out.push(Suggestion {
                    label: "Rust for block",
                    insert: "item in items {\n    \n}",
                });
            }
            if trimmed.ends_with("match ") {
                out.push(Suggestion {
                    label: "Rust match block",
                    insert: "value {\n    _ => {}\n}",
                });
            }
        }
        Language::JavaScript | Language::TypeScript => {
            if trimmed.ends_with("function ") {
                out.push(Suggestion {
                    label: "Function skeleton",
                    insert: "name() {\n    \n}",
                });
            }
            if trimmed.ends_with("if ") {
                out.push(Suggestion {
                    label: "If block",
                    insert: "(condition) {\n    \n}",
                });
            }
            if trimmed.ends_with("for ") {
                out.push(Suggestion {
                    label: "For loop skeleton",
                    insert: "(let i = 0; i < length; i++) {\n    \n}",
                });
            }
            if trimmed.ends_with("imp") || trimmed.ends_with("import ") {
                out.push(Suggestion {
                    label: "ES module import",
                    insert: "{ something } from \"./module\";",
                });
            }
        }
        Language::Python => {
            if trimmed.ends_with("def ") {
                out.push(Suggestion {
                    label: "Python function",
                    insert: "name():\n    ",
                });
            }
            if trimmed.ends_with("class ") {
                out.push(Suggestion {
                    label: "Python class",
                    insert: "Name:\n    def __init__(self) -> None:\n        pass",
                });
            }
            if trimmed.ends_with("if ") {
                out.push(Suggestion {
                    label: "Python if block",
                    insert: "condition:\n    ",
                });
            }
            if trimmed.ends_with("for ") {
                out.push(Suggestion {
                    label: "Python for loop",
                    insert: "item in items:\n    ",
                });
            }
        }
        Language::Json => {
            if trimmed.ends_with('{') {
                out.push(Suggestion {
                    label: "JSON key/value",
                    insert: "\n  \"key\": \"value\"\n}",
                });
                out.push(Suggestion {
                    label: "JSON object with id/name",
                    insert: "\n  \"id\": 1,\n  \"name\": \"example\"\n}",
                });
            }
        }
        _ => {}
    }
    out
}

fn tokenize_html_line(line: &str) -> Vec<Token> {
    let chars: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    let mut index = 0usize;

    while index < chars.len() {
        if starts_with_chars(&chars, index, "<!--") {
            let end = find_sequence(&chars, index + 4, "-->").unwrap_or(chars.len());
            let take = if end < chars.len() { end + 3 } else { end };
            out.push(Token::new(
                slice_chars(&chars, index, take),
                TokenKind::Comment,
            ));
            index = take;
            continue;
        }

        if chars[index] == '<' {
            out.push(Token::new("<".to_string(), TokenKind::Operator));
            index += 1;
            if index < chars.len() && chars[index] == '/' {
                out.push(Token::new("/".to_string(), TokenKind::Operator));
                index += 1;
            }

            let tag_start = index;
            while index < chars.len() && is_html_name_char(chars[index]) {
                index += 1;
            }
            if tag_start < index {
                out.push(Token::new(
                    slice_chars(&chars, tag_start, index),
                    TokenKind::Tag,
                ));
            }

            while index < chars.len() && chars[index] != '>' {
                if chars[index].is_whitespace() {
                    let space_start = index;
                    while index < chars.len() && chars[index].is_whitespace() {
                        index += 1;
                    }
                    out.push(Token::new(
                        slice_chars(&chars, space_start, index),
                        TokenKind::Plain,
                    ));
                    continue;
                }

                if chars[index] == '=' {
                    out.push(Token::new("=".to_string(), TokenKind::Operator));
                    index += 1;
                    continue;
                }

                if chars[index] == '"' || chars[index] == '\'' {
                    let quote = chars[index];
                    let value_start = index;
                    index += 1;
                    while index < chars.len() {
                        if chars[index] == '\\' {
                            index = (index + 2).min(chars.len());
                            continue;
                        }
                        if chars[index] == quote {
                            index += 1;
                            break;
                        }
                        index += 1;
                    }
                    out.push(Token::new(
                        slice_chars(&chars, value_start, index),
                        TokenKind::Value,
                    ));
                    continue;
                }

                let attr_start = index;
                while index < chars.len()
                    && !chars[index].is_whitespace()
                    && chars[index] != '>'
                    && chars[index] != '='
                {
                    index += 1;
                }
                if attr_start < index {
                    out.push(Token::new(
                        slice_chars(&chars, attr_start, index),
                        TokenKind::Attribute,
                    ));
                }
            }

            if index < chars.len() && chars[index] == '>' {
                out.push(Token::new(">".to_string(), TokenKind::Operator));
                index += 1;
            }
            continue;
        }

        let text_start = index;
        while index < chars.len() && chars[index] != '<' {
            index += 1;
        }
        out.push(Token::new(
            slice_chars(&chars, text_start, index),
            TokenKind::Plain,
        ));
    }

    if out.is_empty() {
        out.push(Token::new(String::new(), TokenKind::Plain));
    }
    out
}

fn tokenize_code_line(
    line: &str,
    keywords: &[&str],
    comment_markers: &[&str],
    type_names: &[&str],
) -> Vec<Token> {
    let chars: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    let mut index = 0usize;

    while index < chars.len() {
        if let Some(matched) = starts_with_any(&chars, index, comment_markers) {
            let _ = matched;
            out.push(Token::new(
                slice_chars(&chars, index, chars.len()),
                TokenKind::Comment,
            ));
            break;
        }

        let ch = chars[index];
        if ch.is_whitespace() {
            let start = index;
            while index < chars.len() && chars[index].is_whitespace() {
                index += 1;
            }
            out.push(Token::new(
                slice_chars(&chars, start, index),
                TokenKind::Plain,
            ));
            continue;
        }

        if ch == '"' || ch == '\'' {
            let quote = ch;
            let start = index;
            index += 1;
            while index < chars.len() {
                if chars[index] == '\\' {
                    index = (index + 2).min(chars.len());
                    continue;
                }
                if chars[index] == quote {
                    index += 1;
                    break;
                }
                index += 1;
            }
            out.push(Token::new(
                slice_chars(&chars, start, index),
                TokenKind::String,
            ));
            continue;
        }

        if ch.is_ascii_digit() {
            let start = index;
            index += 1;
            while index < chars.len()
                && (chars[index].is_ascii_digit()
                    || chars[index] == '.'
                    || chars[index] == '_'
                    || chars[index].is_ascii_alphabetic())
            {
                index += 1;
            }
            out.push(Token::new(
                slice_chars(&chars, start, index),
                TokenKind::Number,
            ));
            continue;
        }

        if is_ident_start(ch) {
            let start = index;
            index += 1;
            while index < chars.len() && is_ident_char(chars[index]) {
                index += 1;
            }
            let word = slice_chars(&chars, start, index);
            if keywords.iter().any(|keyword| *keyword == word) {
                out.push(Token::new(word, TokenKind::Keyword));
            } else if type_names.iter().any(|name| *name == word) {
                out.push(Token::new(word, TokenKind::Type));
            } else {
                out.push(Token::new(word, TokenKind::Plain));
            }
            continue;
        }

        if is_operator_char(ch) {
            let start = index;
            index += 1;
            while index < chars.len() && is_operator_char(chars[index]) {
                index += 1;
            }
            out.push(Token::new(
                slice_chars(&chars, start, index),
                TokenKind::Operator,
            ));
            continue;
        }

        out.push(Token::new(ch.to_string(), TokenKind::Plain));
        index += 1;
    }

    if out.is_empty() {
        out.push(Token::new(String::new(), TokenKind::Plain));
    }
    out
}

fn is_html_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':')
}

fn starts_with_chars(chars: &[char], index: usize, needle: &str) -> bool {
    let needle_chars: Vec<char> = needle.chars().collect();
    if index + needle_chars.len() > chars.len() {
        return false;
    }
    for (offset, needle_ch) in needle_chars.iter().enumerate() {
        if chars[index + offset] != *needle_ch {
            return false;
        }
    }
    true
}

fn starts_with_any(chars: &[char], index: usize, needles: &[&str]) -> Option<String> {
    for needle in needles {
        if starts_with_chars(chars, index, needle) {
            return Some((*needle).to_string());
        }
    }
    None
}

fn find_sequence(chars: &[char], start: usize, needle: &str) -> Option<usize> {
    let needle_chars: Vec<char> = needle.chars().collect();
    if needle_chars.is_empty() || start >= chars.len() {
        return None;
    }
    let mut index = start;
    while index + needle_chars.len() <= chars.len() {
        let mut matched = true;
        for (offset, value) in needle_chars.iter().enumerate() {
            if chars[index + offset] != *value {
                matched = false;
                break;
            }
        }
        if matched {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn slice_chars(chars: &[char], start: usize, end: usize) -> String {
    chars[start..end].iter().collect()
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_' || ch == '$'
}

fn is_ident_char(ch: char) -> bool {
    is_ident_start(ch) || ch.is_ascii_digit()
}

fn is_operator_char(ch: char) -> bool {
    matches!(
        ch,
        '{' | '}'
            | '['
            | ']'
            | '('
            | ')'
            | '<'
            | '>'
            | '='
            | '+'
            | '-'
            | '*'
            | '/'
            | '%'
            | '&'
            | '|'
            | '^'
            | '!'
            | '?'
            | ':'
            | ';'
            | ','
            | '.'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_detection_by_extension_works() {
        assert_eq!(detect_language(Path::new("index.html")), Language::Html);
        assert_eq!(detect_language(Path::new("style.css")), Language::Css);
        assert_eq!(detect_language(Path::new("main.rs")), Language::Rust);
        assert_eq!(detect_language(Path::new("app.ts")), Language::TypeScript);
        assert_eq!(detect_language(Path::new("notes.txt")), Language::PlainText);
    }

    #[test]
    fn html_highlight_marks_tags_and_attributes() {
        let tokens = highlight_line(Language::Html, "<div class=\"hero\">");
        assert!(tokens.iter().any(|token| token.kind == TokenKind::Tag));
        assert!(tokens
            .iter()
            .any(|token| token.kind == TokenKind::Attribute));
        assert!(tokens.iter().any(|token| token.kind == TokenKind::Value));
    }

    #[test]
    fn rust_highlight_marks_keywords_and_numbers() {
        let tokens = highlight_line(Language::Rust, "let count = 42;");
        assert!(tokens.iter().any(|token| token.kind == TokenKind::Keyword));
        assert!(tokens.iter().any(|token| token.kind == TokenKind::Number));
    }

    #[test]
    fn suggestions_are_generated_for_known_prefixes() {
        let rust = suggestions_for_line(Language::Rust, "fn ");
        assert!(!rust.is_empty());
        let html = suggestions_for_line(Language::Html, "<");
        assert!(!html.is_empty());
        let plain = suggestions_for_line(Language::PlainText, "hello");
        assert!(plain.is_empty());

        let html_many = suggestions_for_line(Language::Html, "<");
        assert!(html_many.len() >= 2);
    }
}
