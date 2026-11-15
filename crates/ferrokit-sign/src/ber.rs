//! A tiny, defensive BER/DER reader.
//!
//! Signatures found in the wild are not always strict DER (some tools emit indefinite
//! lengths or constructed OCTET STRINGs), and signature checks must run over the *exact*
//! bytes that were signed. So the CMS envelope is walked with this reader, and only
//! leaf structures (certificates, times) are handed to the strict `der` decoders.
//! Every function returns `None` on malformed input; nothing here panics.

/// Deepest nesting accepted for indefinite-length or constructed values.
const MAX_DEPTH: usize = 48;

// Universal tags (identifier octets) used by the callers.
pub const INTEGER: u8 = 0x02;
pub const OCTET_STRING: u8 = 0x04;
pub const OID: u8 = 0x06;
pub const SEQUENCE: u8 = 0x30;
pub const SET: u8 = 0x31;
/// `[0]` constructed, context-specific.
pub const CTX0: u8 = 0xA0;
/// `[0]` primitive, context-specific.
pub const CTX0_PRIMITIVE: u8 = 0x80;

/// One tag-length-value element.
#[derive(Debug, Clone, Copy)]
pub struct Tlv<'a> {
    /// First identifier octet (class, constructed bit and low tag number).
    pub tag: u8,
    /// Content octets (for indefinite lengths: without the end-of-contents marker).
    pub value: &'a [u8],
    /// The whole element: identifier, length, content (and end-of-contents marker).
    pub raw: &'a [u8],
    pub indefinite: bool,
}

/// Parses one element from the start of `input`, returning it and the bytes after it.
pub fn parse(input: &[u8]) -> Option<(Tlv<'_>, &[u8])> {
    parse_at(input, 0)
}

fn parse_at(input: &[u8], depth: usize) -> Option<(Tlv<'_>, &[u8])> {
    if depth > MAX_DEPTH {
        return None;
    }
    let tag = *input.first()?;
    let mut pos = 1;
    if tag & 0x1f == 0x1f {
        // High tag number form: skip the continuation octets.
        loop {
            let b = *input.get(pos)?;
            pos += 1;
            if b & 0x80 == 0 {
                break;
            }
            if pos > 6 {
                return None;
            }
        }
    }
    let first = *input.get(pos)?;
    pos += 1;

    if first == 0x80 {
        // Indefinite length: children until an end-of-contents marker (00 00).
        if tag & 0x20 == 0 {
            return None;
        }
        let start = pos;
        let mut rest = input.get(start..)?;
        loop {
            if rest.len() >= 2 && rest[0] == 0 && rest[1] == 0 {
                let end = input.len() - rest.len();
                let tlv = Tlv { tag, value: &input[start..end], raw: &input[..end + 2], indefinite: true };
                return Some((tlv, &input[end + 2..]));
            }
            let (_, after) = parse_at(rest, depth + 1)?;
            rest = after;
        }
    }

    let len = if first < 0x80 {
        first as usize
    } else {
        let n = (first & 0x7f) as usize;
        if n == 0 || n > 4 {
            return None;
        }
        let mut len = 0usize;
        for _ in 0..n {
            len = (len << 8) | *input.get(pos)? as usize;
            pos += 1;
        }
        len
    };
    let end = pos.checked_add(len)?;
    if end > input.len() {
        return None;
    }
    let tlv = Tlv { tag, value: &input[pos..end], raw: &input[..end], indefinite: false };
    Some((tlv, &input[end..]))
}

impl<'a> Tlv<'a> {
    pub fn is_constructed(&self) -> bool {
        self.tag & 0x20 != 0
    }

    /// The elements inside a constructed value.
    pub fn children(&self) -> Option<Vec<Tlv<'a>>> {
        if !self.is_constructed() {
            return None;
        }
        let mut out = Vec::new();
        let mut rest = self.value;
        while !rest.is_empty() {
            let (child, after) = parse(rest)?;
            out.push(child);
            rest = after;
        }
        Some(out)
    }

    /// Content of an OCTET STRING, reassembling BER constructed strings.
    pub fn octets(&self) -> Option<Vec<u8>> {
        self.octets_at(0)
    }

    fn octets_at(&self, depth: usize) -> Option<Vec<u8>> {
        if depth > MAX_DEPTH {
            return None;
        }
        if !self.is_constructed() {
            return Some(self.value.to_vec());
        }
        let mut out = Vec::new();
        for child in self.children()? {
            out.extend(child.octets_at(depth + 1)?);
        }
        Some(out)
    }

    /// The object identifier in an OID element.
    pub fn oid(&self) -> Option<der::oid::ObjectIdentifier> {
        if self.tag != OID {
            return None;
        }
        der::oid::ObjectIdentifier::from_bytes(self.value).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definite_and_indefinite_lengths() {
        // SEQUENCE (indefinite) { INTEGER 5, OCTET STRING (constructed, indefinite) { "ab", "c" } }
        let ber = [
            0x30, 0x80, 0x02, 0x01, 0x05, 0x24, 0x80, 0x04, 0x02, b'a', b'b', 0x04, 0x01, b'c', 0x00, 0x00, 0x00,
            0x00, 0xff,
        ];
        let (seq, rest) = parse(&ber).unwrap();
        assert!(seq.indefinite);
        assert_eq!(rest, &[0xff]);
        let kids = seq.children().unwrap();
        assert_eq!(kids.len(), 2);
        assert_eq!(kids[0].tag, INTEGER);
        assert_eq!(kids[0].value, &[5]);
        assert_eq!(kids[1].octets().unwrap(), b"abc");
    }

    #[test]
    fn rejects_truncated_and_hostile_input() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0x30]).is_none());
        assert!(parse(&[0x30, 0x05, 0x02, 0x01]).is_none());
        assert!(parse(&[0x30, 0x89, 1, 2, 3, 4, 5, 6, 7, 8, 9]).is_none());
        // Deeply nested indefinite lengths must not overflow the stack.
        let deep: Vec<u8> = std::iter::repeat_n([0x30u8, 0x80], 10_000).flatten().collect();
        assert!(parse(&deep).is_none());
    }
}
