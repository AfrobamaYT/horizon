//! The tokens of Rust source text, each with its line, for [`crate::uses`].
//!
//! A token scan, not a parser: it skips comments (nested block comments included) and
//! whitespace, reads string, byte, C and raw string literals, char and byte literals and number
//! literals with their suffixes as one [`Token::Literal`] each, lifetimes and labels as
//! [`Token::Lifetime`], `r#` raw identifiers by their name, and every other ASCII character as
//! [`Token::Punct`]. Macro bodies are tokens like any other, so a name used only inside
//! `vec![…]` or `format!(…)` is seen. Any text scans: an unterminated literal or comment ends the
//! scan, never a panic.

/// One token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Token<'a> {
    Ident(&'a str),
    /// One ASCII punctuation character: `::` is two.
    Punct(u8),
    Literal,
    Lifetime,
}

/// Every token of `text` with its 1-based line, in order of appearance.
pub fn tokens(text: &str) -> Tokens<'_> {
    Tokens {
        text,
        bytes: text.as_bytes(),
        at: 0,
        line: 1,
    }
}

/// Every identifier of `text` with its 1-based line, in order of appearance.
pub fn identifiers(text: &str) -> impl Iterator<Item = (&str, usize)> {
    tokens(text).filter_map(|(token, line)| match token {
        Token::Ident(name) => Some((name, line)),
        _ => None,
    })
}

/// The iterator [`tokens`] returns.
pub struct Tokens<'a> {
    text: &'a str,
    bytes: &'a [u8],
    at: usize,
    line: usize,
}

/// An identifier byte: ASCII letters, digits and `_`, and every byte of a non-ASCII character
/// (Rust identifiers may hold Unicode letters; the scan takes them whole).
fn is_ident(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte >= 0x80
}

impl<'a> Tokens<'a> {
    fn peek(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.at + offset).copied()
    }

    /// Moves past one byte, counting lines.
    fn bump(&mut self) {
        if self.bytes[self.at] == b'\n' {
            self.line += 1;
        }
        self.at += 1;
    }

    fn word(&mut self) -> &'a str {
        let start = self.at;
        while self.peek(0).is_some_and(is_ident) {
            self.at += 1;
        }
        &self.text[start..self.at]
    }

    /// After `//`: to the end of the line.
    fn line_comment(&mut self) {
        while self.peek(0).is_some_and(|b| b != b'\n') {
            self.at += 1;
        }
    }

    /// After `/*`: to its matching `*/`, nested comments included.
    fn block_comment(&mut self) {
        let mut depth = 1usize;
        while depth > 0 && self.at < self.bytes.len() {
            match (self.peek(0), self.peek(1)) {
                (Some(b'/'), Some(b'*')) => {
                    depth += 1;
                    self.at += 2;
                }
                (Some(b'*'), Some(b'/')) => {
                    depth -= 1;
                    self.at += 2;
                }
                _ => self.bump(),
            }
        }
    }

    /// At the opening `"`: to the closing one, past escapes.
    fn string(&mut self) {
        self.at += 1;
        while let Some(byte) = self.peek(0) {
            match byte {
                b'\\' => {
                    self.at += 1;
                    if self.at < self.bytes.len() {
                        self.bump();
                    }
                }
                b'"' => {
                    self.at += 1;
                    return;
                }
                _ => self.bump(),
            }
        }
    }

    /// At the `#`s or `"` after `r`: to the `"` followed by as many `#`s.
    fn raw_string(&mut self) {
        let mut hashes = 0;
        while self.peek(0) == Some(b'#') {
            hashes += 1;
            self.at += 1;
        }
        self.at += 1; // the opening quote
        while self.at < self.bytes.len() {
            if self.bytes[self.at] == b'"'
                && self.bytes[self.at + 1..]
                    .iter()
                    .take(hashes)
                    .filter(|b| **b == b'#')
                    .count()
                    == hashes
            {
                self.at += 1 + hashes;
                return;
            }
            self.bump();
        }
    }

    /// At `'`: a char or byte literal (`'a'`, `'\n'`, `'é'`) to its closing quote, or a lifetime
    /// or label (`'a`), skipped with its name.
    fn quote(&mut self) -> Token<'a> {
        self.at += 1;
        if self.peek(0) == Some(b'\\') {
            // An escape: only a char literal starts so.
            self.at += 2;
            while self.peek(0).is_some_and(|b| b != b'\'' && b != b'\n') {
                self.at += 1;
            }
            if self.peek(0) == Some(b'\'') {
                self.at += 1;
            }
            return Token::Literal;
        }
        // One character, then a quote: a char literal. Otherwise a lifetime or label.
        let width = self.text[self.at..]
            .chars()
            .next()
            .map_or(0, char::len_utf8);
        if width > 0 && self.peek(width) == Some(b'\'') {
            if self.peek(0) == Some(b'\n') {
                self.line += 1;
            }
            self.at += width + 1;
            Token::Literal
        } else {
            self.word();
            Token::Lifetime
        }
    }
}

impl<'a> Iterator for Tokens<'a> {
    type Item = (Token<'a>, usize);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(byte) = self.peek(0) {
            let line = self.line;
            match byte {
                b'/' if self.peek(1) == Some(b'/') => self.line_comment(),
                b'/' if self.peek(1) == Some(b'*') => {
                    self.at += 2;
                    self.block_comment();
                }
                b'"' => {
                    self.string();
                    return Some((Token::Literal, line));
                }
                b'\'' => return Some((self.quote(), line)),
                b'0'..=b'9' => {
                    // A number with its suffix (`1u32`, `0x_ff`); a `.` ends it, so a tuple
                    // index's field access stays out too.
                    self.word();
                    return Some((Token::Literal, line));
                }
                _ if is_ident(byte) => {
                    let word = self.word();
                    let next = self.peek(0);
                    match (word, next) {
                        ("r" | "br" | "cr", Some(b'"' | b'#'))
                            if next == Some(b'"') || self.raw_string_ahead() =>
                        {
                            self.raw_string();
                            return Some((Token::Literal, line));
                        }
                        ("r", Some(b'#')) => {
                            // A raw identifier: `r#type` is `type`.
                            self.at += 1;
                            let name = self.word();
                            if !name.is_empty() {
                                return Some((Token::Ident(name), line));
                            }
                        }
                        ("b" | "c", Some(b'"')) => {
                            self.string();
                            return Some((Token::Literal, line));
                        }
                        ("b", Some(b'\'')) => return Some((self.quote(), line)),
                        _ => return Some((Token::Ident(word), line)),
                    }
                }
                _ if byte.is_ascii_whitespace() => self.bump(),
                _ => {
                    self.at += 1;
                    return Some((Token::Punct(byte), line));
                }
            }
        }
        None
    }
}

impl Tokens<'_> {
    /// At `#` after `r`, `br` or `cr`: `#`s then `"`, a raw string.
    fn raw_string_ahead(&self) -> bool {
        let rest = &self.bytes[self.at..];
        let hashes = rest.iter().take_while(|b| **b == b'#').count();
        rest.get(hashes) == Some(&b'"')
    }
}
