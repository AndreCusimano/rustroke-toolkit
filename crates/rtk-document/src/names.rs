//! Rust identifiers: validation and case conversion for view and binding names.

/// Rust keywords (strict and reserved), which can't be used as names.
const KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "gen", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut",
    "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
    "unsafe", "use", "where", "while", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "try", "typeof", "unsized", "virtual", "yield",
];

/// Why a name was rejected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NameError {
    /// The name is empty.
    Empty,
    /// The name contains something other than lowercase ASCII letters, digits
    /// and `_`, or starts with a digit.
    NotSnakeCase,
    /// The name is a Rust keyword.
    Keyword,
    /// Another view or field already has this name.
    Duplicate,
}

impl std::fmt::Display for NameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Empty => "the name is empty",
            Self::NotSnakeCase => {
                "use lowercase letters, digits and `_`, not starting with a digit"
            }
            Self::Keyword => "the name is a Rust keyword",
            Self::Duplicate => "the name is already used",
        })
    }
}

impl std::error::Error for NameError {}

/// Checks that `name` is a snake_case Rust identifier usable as a module
/// (file) or field name.
pub fn check_snake_case(name: &str) -> Result<(), NameError> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err(NameError::Empty);
    };
    if !(first.is_ascii_lowercase() || first == '_')
        || !chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        || name == "_"
    {
        return Err(NameError::NotSnakeCase);
    }
    if KEYWORDS.contains(&name) {
        return Err(NameError::Keyword);
    }
    Ok(())
}

/// Whether `name` is any valid Rust identifier (not a keyword).
pub fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_alphabetic() || first == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_')
        && name != "_"
        && !KEYWORDS.contains(&name)
}

/// `login_form` → `LoginForm` (struct name of a view).
pub fn to_pascal_case(snake: &str) -> String {
    snake
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// Turns free text (`"Login form"`, `"LoginForm"`) into a snake_case name
/// (`login_form`), for suggestions. The result may still be a keyword or
/// empty; check it with [`check_snake_case`].
pub fn to_snake_case(text: &str) -> String {
    let mut out = String::new();
    let mut prev_lower = false;
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            if c.is_ascii_uppercase() && prev_lower && !out.ends_with('_') {
                out.push('_');
            }
            prev_lower = c.is_ascii_lowercase();
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('_') {
            out.push('_');
            prev_lower = false;
        }
    }
    let trimmed = out.trim_end_matches('_');
    if trimmed.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{trimmed}")
    } else {
        trimmed.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_case_names() {
        assert_eq!(check_snake_case("login_form"), Ok(()));
        assert_eq!(check_snake_case("_private2"), Ok(()));
        assert_eq!(check_snake_case(""), Err(NameError::Empty));
        assert_eq!(check_snake_case("Login"), Err(NameError::NotSnakeCase));
        assert_eq!(check_snake_case("2fa"), Err(NameError::NotSnakeCase));
        assert_eq!(check_snake_case("a-b"), Err(NameError::NotSnakeCase));
        assert_eq!(check_snake_case("_"), Err(NameError::NotSnakeCase));
        assert_eq!(check_snake_case("type"), Err(NameError::Keyword));
    }

    #[test]
    fn conversions() {
        assert_eq!(to_pascal_case("login_form"), "LoginForm");
        assert_eq!(to_pascal_case("view2"), "View2");
        assert_eq!(to_snake_case("Login form"), "login_form");
        assert_eq!(to_snake_case("LoginForm"), "login_form");
        assert_eq!(to_snake_case("  3D view! "), "_3d_view");
        assert!(is_identifier("Login"));
        assert!(!is_identifier("fn"));
        assert!(!is_identifier("a b"));
    }
}
