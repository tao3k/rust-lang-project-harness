use quote::ToTokens;

pub(super) fn compact_tokens(value: &impl ToTokens) -> String {
    compact_rust_tokens(&value.to_token_stream().to_string())
}

pub(super) fn compact_limited(value: &str, max_len: usize) -> String {
    if value.len() <= max_len {
        return value.to_string();
    }
    let mut truncated = value
        .chars()
        .take(max_len.saturating_sub(3))
        .collect::<String>();
    truncated.push_str("...");
    truncated
}

fn compact_rust_tokens(value: &str) -> String {
    #[derive(Clone, Copy)]
    enum LiteralKind {
        String,
        ByteString,
        RawString,
        RawByteString,
        Char,
        ByteChar,
    }

    fn literal_kind_label(kind: LiteralKind) -> &'static str {
        match kind {
            LiteralKind::String => "string",
            LiteralKind::ByteString => "byte-string",
            LiteralKind::RawString => "raw-string",
            LiteralKind::RawByteString => "raw-byte-string",
            LiteralKind::Char => "char",
            LiteralKind::ByteChar => "byte-char",
        }
    }

    fn literal_hash(value: &str) -> String {
        let mut hash = 2_166_136_261u32;
        for byte in value.as_bytes() {
            hash ^= u32::from(*byte);
            hash = hash.wrapping_mul(16_777_619);
        }
        format!("{hash:x}")
    }

    fn literal_projection(kind: LiteralKind, token: &str) -> String {
        if !token.chars().any(char::is_whitespace) {
            return token.to_string();
        }
        let lines = token.bytes().filter(|byte| *byte == b'\n').count() + 1;
        format!(
            "{}[lines={},bytes={},hash={}]",
            literal_kind_label(kind),
            lines,
            token.len(),
            literal_hash(token)
        )
    }

    fn take_raw_literal(
        rest: &str,
        prefix_len: usize,
        kind: LiteralKind,
    ) -> Option<(usize, LiteralKind)> {
        let bytes = rest.as_bytes();
        let mut cursor = prefix_len;
        while bytes.get(cursor).is_some_and(|byte| *byte == b'#') {
            cursor += 1;
        }
        if bytes.get(cursor) != Some(&b'"') {
            return None;
        }
        let hashes = cursor.saturating_sub(prefix_len);
        cursor += 1;
        while cursor < bytes.len() {
            if bytes[cursor] == b'"'
                && cursor + 1 + hashes <= bytes.len()
                && bytes[cursor + 1..cursor + 1 + hashes]
                    .iter()
                    .all(|byte| *byte == b'#')
            {
                return Some((cursor + 1 + hashes, kind));
            }
            cursor += 1;
        }
        None
    }

    fn take_quoted_literal(
        rest: &str,
        prefix_len: usize,
        quote: u8,
        kind: LiteralKind,
    ) -> Option<(usize, LiteralKind)> {
        if rest.as_bytes().get(prefix_len) != Some(&quote) {
            return None;
        }
        let mut cursor = prefix_len + 1;
        let mut escaped = false;
        while cursor < rest.len() {
            let ch = rest[cursor..].chars().next()?;
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch.len_utf8() == 1 && ch as u8 == quote {
                return Some((cursor + 1, kind));
            }
            cursor += ch.len_utf8();
        }
        None
    }

    fn take_char_literal(
        rest: &str,
        prefix_len: usize,
        kind: LiteralKind,
    ) -> Option<(usize, LiteralKind)> {
        let (literal_len, kind) = take_quoted_literal(rest, prefix_len, b'\'', kind)?;
        let body = &rest[prefix_len + 1..literal_len - 1];
        (body.starts_with('\\') || body.chars().count() == 1).then_some((literal_len, kind))
    }

    fn take_literal(rest: &str) -> Option<(usize, LiteralKind)> {
        take_raw_literal(rest, 2, LiteralKind::RawByteString)
            .filter(|_| rest.starts_with("br"))
            .or_else(|| {
                take_raw_literal(rest, 2, LiteralKind::RawString).filter(|_| rest.starts_with("cr"))
            })
            .or_else(|| {
                take_raw_literal(rest, 1, LiteralKind::RawString).filter(|_| rest.starts_with('r'))
            })
            .or_else(|| {
                take_quoted_literal(rest, 1, b'"', LiteralKind::ByteString)
                    .filter(|_| rest.starts_with('b'))
            })
            .or_else(|| {
                take_quoted_literal(rest, 1, b'"', LiteralKind::String)
                    .filter(|_| rest.starts_with('c'))
            })
            .or_else(|| {
                take_char_literal(rest, 1, LiteralKind::ByteChar).filter(|_| rest.starts_with('b'))
            })
            .or_else(|| take_quoted_literal(rest, 0, b'"', LiteralKind::String))
            .or_else(|| take_char_literal(rest, 0, LiteralKind::Char))
    }

    let mut literal_safe = String::with_capacity(value.len());
    let mut literals = Vec::<String>::new();
    let mut cursor = 0;
    while cursor < value.len() {
        let rest = &value[cursor..];
        if let Some((literal_len, kind)) = take_literal(rest) {
            let placeholder = format!("__ASP_LITERAL_{}__", literals.len());
            literals.push(literal_projection(kind, &rest[..literal_len]));
            literal_safe.push_str(&placeholder);
            cursor += literal_len;
        } else if let Some(ch) = rest.chars().next() {
            literal_safe.push(ch);
            cursor += ch.len_utf8();
        } else {
            break;
        }
    }

    let mut compacted = literal_safe
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for (from, to) in [
        (" :: ", "::"),
        (" (", "("),
        ("( ", "("),
        (" )", ")"),
        (" [", "["),
        ("[ ", "["),
        (" ]", "]"),
        (" ,", ","),
        (",)", ")"),
        (" ;", ";"),
        (" :", ":"),
        (" & ", "&"),
        ("& ", "&"),
        (" * ", " *"),
        (" !", "!"),
        (" . ", "."),
        (" <", "<"),
        ("< ", "<"),
        (" >", ">"),
        (" < ", "<"),
        (" > ", ">"),
        (":&", ": &"),
        ("->&", "-> &"),
        ("->(", "-> ("),
    ] {
        compacted = compacted.replace(from, to);
    }
    for (index, literal) in literals.iter().enumerate() {
        compacted = compacted.replace(&format!("__ASP_LITERAL_{index}__"), literal);
    }
    compacted
}
