//! Rust syntax colors for the code editor: comments, strings, keywords,
//! numbers, types and macros. A simple scanner, not a parser: enough for
//! the code the toolkit generates and for hand-written additions.

use rustroke::{Color, LayoutJob, TextFormat};

/// Colors of the token classes.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    comment: Color,
    string: Color,
    keyword: Color,
    number: Color,
    ty: Color,
    mac: Color,
}

impl Palette {
    /// For a dark or a light background.
    pub fn new(dark: bool) -> Self {
        if dark {
            Self {
                comment: Color::from_srgb8(128, 134, 140),
                string: Color::from_srgb8(152, 195, 121),
                keyword: Color::from_srgb8(198, 120, 221),
                number: Color::from_srgb8(209, 154, 102),
                ty: Color::from_srgb8(229, 192, 123),
                mac: Color::from_srgb8(97, 175, 239),
            }
        } else {
            Self {
                comment: Color::from_srgb8(120, 120, 120),
                string: Color::from_srgb8(56, 125, 40),
                keyword: Color::from_srgb8(150, 40, 170),
                number: Color::from_srgb8(170, 90, 20),
                ty: Color::from_srgb8(150, 100, 0),
                mac: Color::from_srgb8(30, 100, 190),
            }
        }
    }
}

const KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "dyn", "else", "enum", "false", "fn", "for", "if",
    "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self",
    "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while",
];

/// `code` split into colored sections; the sections hold exactly `code`.
pub fn highlight(code: &str, palette: Palette) -> LayoutJob {
    let mut job = LayoutJob::default();
    let plain = TextFormat::new();
    let colored = |c: Color| TextFormat::new().color(c);
    let bytes = code.as_bytes();
    let mut i = 0;
    // Start of the plain text not appended yet.
    let mut plain_start = 0;
    let push = |job: &mut LayoutJob, plain_start: &mut usize, start, end, format: TextFormat| {
        if *plain_start < start {
            job.append(&code[*plain_start..start], plain.clone());
        }
        job.append(&code[start..end], format);
        *plain_start = end;
    };
    while i < bytes.len() {
        let c = bytes[i];
        let start = i;
        if code[i..].starts_with("//") {
            i = code[i..].find('\n').map_or(code.len(), |n| i + n);
            push(
                &mut job,
                &mut plain_start,
                start,
                i,
                colored(palette.comment),
            );
        } else if c == b'"' {
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                i += if bytes[i] == b'\\' { 2 } else { 1 };
            }
            i = (i + 1).min(bytes.len());
            push(
                &mut job,
                &mut plain_start,
                start,
                i,
                colored(palette.string),
            );
        } else if c == b'\'' && is_char_literal(&bytes[i..]) {
            i += 1;
            while i < bytes.len() && bytes[i] != b'\'' {
                i += if bytes[i] == b'\\' { 2 } else { 1 };
            }
            i = (i + 1).min(bytes.len());
            push(
                &mut job,
                &mut plain_start,
                start,
                i,
                colored(palette.string),
            );
        } else if c.is_ascii_digit() {
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            // A fraction, not a method call or a range.
            if i + 1 < bytes.len() && bytes[i] == b'.' && bytes[i + 1].is_ascii_digit() {
                i += 1;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
            }
            push(
                &mut job,
                &mut plain_start,
                start,
                i,
                colored(palette.number),
            );
        } else if c.is_ascii_alphabetic() || c == b'_' {
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let word = &code[start..i];
            let format = if bytes.get(i) == Some(&b'!') {
                i += 1;
                Some(palette.mac)
            } else if KEYWORDS.contains(&word) {
                Some(palette.keyword)
            } else if word.starts_with(|c: char| c.is_ascii_uppercase()) {
                Some(palette.ty)
            } else {
                None
            };
            if let Some(color) = format {
                push(&mut job, &mut plain_start, start, i, colored(color));
            }
        } else {
            // Any other character, whole (it may be several bytes long).
            i += code[i..].chars().next().map_or(1, char::len_utf8);
        }
    }
    if plain_start < code.len() {
        job.append(&code[plain_start..], plain);
    }
    job
}

/// Whether the `'` at the start of `bytes` opens a char literal (not a
/// lifetime like `'_` or `'a`).
fn is_char_literal(bytes: &[u8]) -> bool {
    match bytes {
        [b'\'', b'\\', ..] => true,
        [b'\'', _, b'\'', ..] => true,
        // A multi-byte character.
        [b'\'', first, rest @ ..] if *first >= 0x80 => rest.iter().take(4).any(|b| *b == b'\''),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_hold_the_whole_text() {
        let code = "// é comment\nfn show(&mut self) { let x = 1.5; ui.label(\"a \\\" b\"); vec![x]; 'a'; '_; Label::new(\"日本\") }\n\"unterminated";
        let job = highlight(code, Palette::new(true));
        assert_eq!(job.text, code);
        let texts: Vec<&str> = job
            .sections
            .iter()
            .map(|(range, _)| &job.text[range.clone()])
            .collect();
        for expected in [
            "// é comment",
            "fn",
            "1.5",
            "\"a \\\" b\"",
            "vec!",
            "'a'",
            "Label",
        ] {
            assert!(texts.contains(&expected), "{expected:?} in {texts:?}");
        }
        assert!(!texts.contains(&"'_"), "a lifetime is not a char");
    }
}
