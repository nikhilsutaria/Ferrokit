//! Metrics for the standard Helvetica font (no embedding needed: every PDF viewer has it).

/// Helvetica advance widths (1/1000 em) for WinAnsi codes 32..=126.
const HELVETICA: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, // space ! " # $ % & ' ( ) * + , - . /
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, // 0-9
    278, 278, 584, 584, 584, 556, 1015, // : ; < = > ? @
    667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, // A-M
    722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, // N-Z
    278, 278, 278, 469, 556, 333, // [ \ ] ^ _ `
    556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, // a-m
    556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, // n-z
    334, 260, 334, 584, // { | } ~
];

/// Encode text as WinAnsi bytes. Latin-1 characters map directly; anything else becomes `?`.
pub fn win_ansi(text: &str) -> Vec<u8> {
    text.chars()
        .map(|c| match c as u32 {
            0x20..=0x7E | 0xA0..=0xFF => c as u32 as u8,
            0x2013 => 0x96, // en dash
            0x2014 => 0x97, // em dash
            0x2018 => 0x91,
            0x2019 => 0x92,
            0x201C => 0x93,
            0x201D => 0x94,
            0x2022 => 0x95, // bullet
            0x20AC => 0x80, // euro
            0x2026 => 0x85, // ellipsis
            _ => b'?',
        })
        .collect()
}

/// Width of WinAnsi-encoded text in points at the given font size.
pub fn width(bytes: &[u8], size: f64) -> f64 {
    let units: u32 = bytes
        .iter()
        .map(|&b| match b {
            32..=126 => HELVETICA[(b - 32) as usize] as u32,
            _ => 556,
        })
        .sum();
    units as f64 * size / 1000.0
}

/// Escape bytes for a PDF literal string `( … )`.
pub fn literal(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() + 2);
    out.push(b'(');
    for &b in bytes {
        match b {
            b'(' | b')' | b'\\' => {
                out.push(b'\\');
                out.push(b);
            }
            b'\r' => out.extend_from_slice(b"\\r"),
            b'\n' => out.extend_from_slice(b"\\n"),
            _ => out.push(b),
        }
    }
    out.push(b')');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths() {
        assert_eq!(width(b"0", 1000.0), 556.0);
        assert_eq!(width(b"Page 1", 10.0), (667 + 556 + 556 + 556 + 278 + 556) as f64 / 100.0);
    }

    #[test]
    fn encoding_and_escaping() {
        assert_eq!(win_ansi("Café – €"), vec![b'C', b'a', b'f', 0xE9, b' ', 0x96, b' ', 0x80]);
        assert_eq!(win_ansi("日"), b"?");
        assert_eq!(literal(b"a(b)\\"), b"(a\\(b\\)\\\\)".to_vec());
    }
}
