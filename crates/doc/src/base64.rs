//! Base64 (RFC 4648: the standard alphabet, padded), as JSON carries binary data: in native files,
//! the plug-in ABI and the control channel. It lives here, at the bottom of the crate stack, so
//! every crate has the same one: `vectorcraft-format` re-exports it as `base64_encode` and
//! `base64_decode`, `vectorcraft-svg` its encoder, and the Glass effect's loaded textures use it.

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// `data` as padded base64.
pub fn encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let byte = |i: usize| u32::from(c.get(i).copied().unwrap_or(0));
        let n = byte(0) << 16 | byte(1) << 8 | byte(2);
        let digit = |shift: u32| char::from(ALPHABET[(n >> shift) as usize & 63]);
        out.push(digit(18));
        out.push(digit(12));
        out.push(if c.len() > 1 { digit(6) } else { '=' });
        out.push(if c.len() > 2 { digit(0) } else { '=' });
    }
    out
}

/// The bytes padded base64 `s` holds (white space ignored), or `None` when it isn't base64.
pub fn decode(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| ALPHABET.iter().position(|&b| b == c).map(|p| p as u32);
    let clean: Vec<u8> = s.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if !clean.len().is_multiple_of(4) {
        return None;
    }
    let mut out = Vec::with_capacity(clean.len() / 4 * 3);
    for c in clean.chunks(4) {
        let mut n = 0u32;
        let mut pad = 0;
        for &b in c {
            n <<= 6;
            if b == b'=' {
                pad += 1;
            } else {
                n |= val(b)?;
            }
        }
        out.push((n >> 16) as u8);
        if pad < 2 {
            out.push((n >> 8) as u8);
        }
        if pad < 1 {
            out.push(n as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_every_length_modulo_three() {
        for data in [&b""[..], b"f", b"fo", b"foo", b"foob", b"\x00\xff\x10"] {
            assert_eq!(decode(&encode(data)).as_deref(), Some(data));
        }
        assert_eq!(encode(b"foobar"), "Zm9vYmFy");
        assert_eq!(decode("Zm9v\nYmE="), Some(b"fooba".to_vec()), "white space is ignored");
        assert_eq!(decode("Zm9"), None, "not a multiple of four");
        assert_eq!(decode("Zm9!"), None, "not the alphabet");
    }
}
