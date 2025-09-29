//! GSM 7-bit character encoding and decoding library.
//!
//! This library provides efficient encoding and decoding of text using the GSM 7-bit
//! character set as defined in GSM 03.38. This encoding is commonly used in SMS messages.
//!
//! # Example
//!
//! ```rust
//! use gsm7::{encode, decode};
//!
//! let text = "Hello {world} €!";
//! let encoded = encode(text)?;
//! let decoded = decode(&encoded)?;
//! assert_eq!(decoded, text);
//! # Ok::<(), gsm7::Gsm7Error>(())
//! ```

use once_cell::sync::Lazy;
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during GSM 7-bit encoding/decoding operations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum Gsm7Error {
    /// Character is not supported in GSM 7-bit encoding.
    #[error("Character not supported in GSM 7-bit: '{character}' (U+{code:04X})")]
    UnsupportedCharacter { character: char, code: u32 },

    /// Invalid escape sequence encountered during decoding.
    #[error("Invalid escape sequence: 0x1B followed by 0x{code:02X}")]
    InvalidEscapeSequence { code: u8 },

    /// Invalid byte encountered during decoding.
    #[error("Invalid GSM 7-bit byte: 0x{byte:02X}")]
    InvalidByte { byte: u8 },

    /// Input data is malformed.
    #[error("Malformed GSM 7-bit data: {reason}")]
    MalformedData { reason: String },
}

/// Result type for GSM 7-bit operations.
pub type Result<T> = std::result::Result<T, Gsm7Error>;

/// Configuration options for GSM 7-bit encoding/decoding.
#[derive(Debug, Clone)]
pub struct Gsm7Config {
    /// Whether to use strict mode (fail on unsupported characters) or replace them.
    pub strict: bool,
    /// Replacement character for unsupported characters in non-strict mode.
    pub replacement_char: char,
}

impl Default for Gsm7Config {
    fn default() -> Self {
        Self {
            strict: false,
            replacement_char: '�',
        }
    }
}

impl Gsm7Config {
    /// Create a config with strict mode enabled.
    pub fn strict() -> Self {
        Self {
            strict: true,
            replacement_char: '�',
        }
    }
}

/// Internal representation of GSM 7-bit codes.
#[derive(Debug, Clone)]
enum Code {
    /// Single byte code.
    Single(u8),
    /// Escape sequence (0x1B followed by another byte).
    Escape(u8),
}

/// GSM 7-bit character mappings (lazy-initialized static data).
static GSM_MAPS: Lazy<(HashMap<char, Code>, [Option<char>; 128], HashMap<u8, char>)> =
    Lazy::new(|| {
        let gsm_to_char = build_gsm_table();
        let gsm_ext = build_gsm_ext_table();

        let mut char_to_gsm = HashMap::new();
        let mut gsm_array = [None; 128];

        // Build character to GSM mapping and array for fast lookup
        for (&code, &ch) in &gsm_to_char {
            if let Some(character) = ch {
                char_to_gsm.insert(character, Code::Single(code));
                gsm_array[code as usize] = Some(character);
            }
        }

        // Add extension characters
        for (&code, &ch) in &gsm_ext {
            char_to_gsm.insert(ch, Code::Escape(code));
        }

        (char_to_gsm, gsm_array, gsm_ext)
    });

/// Encode a string using GSM 7-bit encoding.
///
/// # Arguments
///
/// * `content` - The string to encode
///
/// # Returns
///
/// A `Vec<u8>` containing the GSM 7-bit encoded bytes.
///
/// # Errors
///
/// Returns `Gsm7Error::UnsupportedCharacter` if the input contains characters
/// not supported by the GSM 7-bit character set.
///
/// # Example
///
/// ```rust
/// use gsm7::encode;
///
/// let encoded = encode("Hello World!")?;
/// assert!(!encoded.is_empty());
/// # Ok::<(), gsm7::Gsm7Error>(())
/// ```
pub fn encode(content: &str) -> Result<Vec<u8>> {
    encode_with_config(content, &Gsm7Config::default())
}

/// Encode a string using GSM 7-bit encoding with custom configuration.
///
/// # Arguments
///
/// * `content` - The string to encode
/// * `config` - Configuration options
///
/// # Returns
///
/// A `Vec<u8>` containing the GSM 7-bit encoded bytes.
pub fn encode_with_config(content: &str, config: &Gsm7Config) -> Result<Vec<u8>> {
    let (char_to_gsm, _, _) = &*GSM_MAPS;
    let mut bytes = Vec::with_capacity(content.len());

    for ch in content.chars() {
        match char_to_gsm.get(&ch) {
            Some(Code::Single(b)) => bytes.push(*b),
            Some(Code::Escape(b)) => {
                bytes.push(0x1B);
                bytes.push(*b);
            }
            None => {
                if config.strict {
                    return Err(Gsm7Error::UnsupportedCharacter {
                        character: ch,
                        code: ch as u32,
                    });
                } else {
                    // Use replacement character
                    if let Some(Code::Single(b)) = char_to_gsm.get(&config.replacement_char) {
                        bytes.push(*b);
                    } else {
                        bytes.push(0x20); // space as fallback
                    }
                }
            }
        }
    }

    Ok(bytes)
}

/// Decode GSM 7-bit encoded bytes to a string.
///
/// # Arguments
///
/// * `data` - The GSM 7-bit encoded bytes to decode
///
/// # Returns
///
/// A `String` containing the decoded text. Invalid bytes are replaced with '�'.
///
/// # Note
///
/// This function uses non-strict mode by default. Invalid bytes will be replaced
/// with the replacement character '�' instead of returning an error.
///
/// # Example
///
/// ```rust
/// use gsm7::{encode, decode};
///
/// let original = "Hello World!";
/// let encoded = encode(original)?;
/// let decoded = decode(&encoded)?;
/// assert_eq!(decoded, original);
///
/// // Invalid bytes are replaced
/// let invalid_data = vec![0x48, 0x81, 0x65]; // "H" + invalid + "e"
/// let decoded = decode(&invalid_data)?;
/// assert_eq!(decoded, "H�e");
/// # Ok::<(), gsm7::Gsm7Error>(())
/// ```
pub fn decode(data: &[u8]) -> Result<String> {
    decode_with_config(data, &Gsm7Config::default())
}

/// Decode GSM 7-bit encoded bytes to a string with custom configuration.
///
/// # Arguments
///
/// * `data` - The GSM 7-bit encoded bytes to decode
/// * `config` - Configuration options
///
/// # Returns
///
/// A `String` containing the decoded text.
pub fn decode_with_config(data: &[u8], config: &Gsm7Config) -> Result<String> {
    let (_, gsm_array, gsm_ext) = &*GSM_MAPS;
    let mut result = String::with_capacity(data.len());

    let mut i = 0;
    while i < data.len() {
        let code = data[i];

        if code == 0x1B {
            // Handle escape sequence
            if let Some(&next_code) = data.get(i + 1) {
                match gsm_ext.get(&next_code) {
                    Some(&ch) => {
                        result.push(ch);
                        i += 2;
                        continue;
                    }
                    None => {
                        if config.strict {
                            return Err(Gsm7Error::InvalidEscapeSequence { code: next_code });
                        } else {
                            result.push(config.replacement_char);
                            i += 2;
                            continue;
                        }
                    }
                }
            } else {
                if config.strict {
                    return Err(Gsm7Error::MalformedData {
                        reason: "Escape byte at end of input".to_string(),
                    });
                } else {
                    result.push(config.replacement_char);
                    i += 1;
                    continue;
                }
            }
        } else if code < 128 {
            // Handle regular character
            match gsm_array[code as usize] {
                Some(ch) => result.push(ch),
                None => {
                    if config.strict {
                        return Err(Gsm7Error::InvalidByte { byte: code });
                    } else {
                        result.push(config.replacement_char);
                    }
                }
            }
        } else {
            // Invalid byte (>= 128) - always replace with � character
            result.push(config.replacement_char);
        }
        i += 1;
    }

    Ok(result)
}

/// Calculate the number of bytes required to encode a string in GSM 7-bit.
///
/// This is useful for SMS length calculations.
///
/// # Arguments
///
/// * `content` - The string to measure
///
/// # Returns
///
/// The number of bytes required, or an error if the string contains
/// unsupported characters.
pub fn encoded_len(content: &str) -> Result<usize> {
    let (char_to_gsm, _, _) = &*GSM_MAPS;
    let mut len = 0;

    for ch in content.chars() {
        match char_to_gsm.get(&ch) {
            Some(Code::Single(_)) => len += 1,
            Some(Code::Escape(_)) => len += 2,
            None => {
                return Err(Gsm7Error::UnsupportedCharacter {
                    character: ch,
                    code: ch as u32,
                });
            }
        }
    }

    Ok(len)
}

/// Check if a string can be encoded in GSM 7-bit without errors.
///
/// # Arguments
///
/// * `content` - The string to check
///
/// # Returns
///
/// `true` if the string can be encoded, `false` otherwise.
pub fn is_gsm7_compatible(content: &str) -> bool {
    encoded_len(content).is_ok()
}

/// Base GSM 7-bit character table as defined in GSM 03.38.
fn build_gsm_table() -> HashMap<u8, Option<char>> {
    let mut map = HashMap::new();

    let table: &[(u8, Option<char>)] = &[
        (0x00, Some('@')),
        (0x01, Some('£')),
        (0x02, Some('$')),
        (0x03, Some('¥')),
        (0x04, Some('è')),
        (0x05, Some('é')),
        (0x06, Some('ù')),
        (0x07, Some('ì')),
        (0x08, Some('ò')),
        (0x09, Some('Ç')),
        (0x0A, Some('\n')),
        (0x0B, Some('Ø')),
        (0x0C, Some('ø')),
        (0x0D, Some('\r')),
        (0x0E, Some('Å')),
        (0x0F, Some('å')),
        (0x10, Some('Δ')),
        (0x11, Some('_')),
        (0x12, Some('Φ')),
        (0x13, Some('Γ')),
        (0x14, Some('Λ')),
        (0x15, Some('Ω')),
        (0x16, Some('Π')),
        (0x17, Some('Ψ')),
        (0x18, Some('Σ')),
        (0x19, Some('Θ')),
        (0x1A, Some('Ξ')),
        (0x1B, None), // ESC - no character representation
        (0x1C, Some('Æ')),
        (0x1D, Some('æ')),
        (0x1E, Some('ß')),
        (0x1F, Some('É')),
        (0x20, Some(' ')),
        (0x21, Some('!')),
        (0x22, Some('"')),
        (0x23, Some('#')),
        (0x24, Some('¤')),
        (0x25, Some('%')),
        (0x26, Some('&')),
        (0x27, Some('\'')),
        (0x28, Some('(')),
        (0x29, Some(')')),
        (0x2A, Some('*')),
        (0x2B, Some('+')),
        (0x2C, Some(',')),
        (0x2D, Some('-')),
        (0x2E, Some('.')),
        (0x2F, Some('/')),
        (0x30, Some('0')),
        (0x31, Some('1')),
        (0x32, Some('2')),
        (0x33, Some('3')),
        (0x34, Some('4')),
        (0x35, Some('5')),
        (0x36, Some('6')),
        (0x37, Some('7')),
        (0x38, Some('8')),
        (0x39, Some('9')),
        (0x3A, Some(':')),
        (0x3B, Some(';')),
        (0x3C, Some('<')),
        (0x3D, Some('=')),
        (0x3E, Some('>')),
        (0x3F, Some('?')),
        (0x40, Some('¡')),
        (0x41, Some('A')),
        (0x42, Some('B')),
        (0x43, Some('C')),
        (0x44, Some('D')),
        (0x45, Some('E')),
        (0x46, Some('F')),
        (0x47, Some('G')),
        (0x48, Some('H')),
        (0x49, Some('I')),
        (0x4A, Some('J')),
        (0x4B, Some('K')),
        (0x4C, Some('L')),
        (0x4D, Some('M')),
        (0x4E, Some('N')),
        (0x4F, Some('O')),
        (0x50, Some('P')),
        (0x51, Some('Q')),
        (0x52, Some('R')),
        (0x53, Some('S')),
        (0x54, Some('T')),
        (0x55, Some('U')),
        (0x56, Some('V')),
        (0x57, Some('W')),
        (0x58, Some('X')),
        (0x59, Some('Y')),
        (0x5A, Some('Z')),
        (0x5B, Some('Ä')),
        (0x5C, Some('Ö')),
        (0x5D, Some('Ñ')),
        (0x5E, Some('Ü')),
        (0x5F, Some('§')),
        (0x60, Some('¿')),
        (0x61, Some('a')),
        (0x62, Some('b')),
        (0x63, Some('c')),
        (0x64, Some('d')),
        (0x65, Some('e')),
        (0x66, Some('f')),
        (0x67, Some('g')),
        (0x68, Some('h')),
        (0x69, Some('i')),
        (0x6A, Some('j')),
        (0x6B, Some('k')),
        (0x6C, Some('l')),
        (0x6D, Some('m')),
        (0x6E, Some('n')),
        (0x6F, Some('o')),
        (0x70, Some('p')),
        (0x71, Some('q')),
        (0x72, Some('r')),
        (0x73, Some('s')),
        (0x74, Some('t')),
        (0x75, Some('u')),
        (0x76, Some('v')),
        (0x77, Some('w')),
        (0x78, Some('x')),
        (0x79, Some('y')),
        (0x7A, Some('z')),
        (0x7B, Some('ä')),
        (0x7C, Some('ö')),
        (0x7D, Some('ñ')),
        (0x7E, Some('ü')),
        (0x7F, Some('à')),
    ];

    for &(code, ch) in table {
        map.insert(code, ch);
    }

    map
}

/// GSM 7-bit extension table (characters prefixed with 0x1B).
fn build_gsm_ext_table() -> HashMap<u8, char> {
    let mut map = HashMap::new();

    let table: &[(u8, char)] = &[
        (0x0A, '\x0C'), // Form feed
        (0x14, '^'),
        (0x28, '{'),
        (0x29, '}'),
        (0x2F, '\\'),
        (0x3C, '['),
        (0x3D, '~'),
        (0x3E, ']'),
        (0x40, '|'),
        (0x65, '€'),
    ];

    for &(code, ch) in table {
        map.insert(code, ch);
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let test_cases = [
            "Hello World!",
            "Hello {world} €!",
            "GSM 7-bit test: àáâãäåæçèéêë",
            "Greek letters: ΔΦΓΛΩΠΨΣΘΞ",
            "Special chars: @£$¥èé",
            "Extension chars: {[]}\\~€|^",
            "Numbers: 0123456789",
            "Punctuation: !\"#¤%&'()*+,-./:;<=>?¡¿§",
        ];

        for text in &test_cases {
            let encoded = encode(text).unwrap();
            let decoded = decode(&encoded).unwrap();
            assert_eq!(decoded, *text, "Failed roundtrip for: {}", text);
        }
    }

    #[test]
    fn test_unsupported_character() {
        // With default config (non-strict), should replace with �
        let encoded = encode("Hello 🦀 World").unwrap();
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, "Hello � World");
    }

    #[test]
    fn test_empty_string() {
        let encoded = encode("").unwrap();
        assert!(encoded.is_empty());
        let decoded = decode(&encoded).unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn test_encoded_len() {
        assert_eq!(encoded_len("Hello").unwrap(), 5);
        assert_eq!(encoded_len("Hello €").unwrap(), 7); // € is 2 bytes
        assert_eq!(encoded_len("{[]}").unwrap(), 8); // All extension chars
    }

    #[test]
    fn test_is_gsm7_compatible() {
        assert!(is_gsm7_compatible("Hello World!"));
        assert!(is_gsm7_compatible("Hello {world} €!"));
        assert!(!is_gsm7_compatible("Hello 🦀 World"));
    }

    #[test]
    fn test_non_strict_mode() {
        let config = Gsm7Config {
            strict: false,
            replacement_char: '?',
        };

        let encoded = encode_with_config("Hello 🦀 World", &config).unwrap();
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, "Hello ? World");
    }

    #[test]
    fn test_invalid_byte_replaced() {
        // Test byte 0x81 (outside valid range)
        let invalid_data = vec![0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x81, 0x20, 0x57]; // "Hello" + 0x81 + " W"
        let decoded = decode(&invalid_data).unwrap();
        assert_eq!(decoded, "Hello� W");
    }

    #[test]
    fn test_multiple_invalid_bytes() {
        // Test multiple invalid bytes interspersed with valid ones
        let invalid_data = vec![0x48, 0x81, 0x65, 0x82, 0x6C, 0x83]; // "H" + 0x81 + "e" + 0x82 + "l" + 0x83
        let decoded = decode(&invalid_data).unwrap();
        assert_eq!(decoded, "H�e�l�");
    }

    #[test]
    fn test_invalid_escape_sequence() {
        // Create invalid data: escape followed by unsupported code
        let invalid_data = [0x1B, 0xFF];
        let decoded = decode(&invalid_data).unwrap();
        assert_eq!(decoded, "�");
    }

    #[test]
    fn test_malformed_data() {
        // Escape at end of input - should replace with �
        let invalid_data = [0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x1B];
        let decoded = decode(&invalid_data).unwrap();
        assert_eq!(decoded, "Hello�");
    }

    #[test]
    fn test_all_basic_characters() {
        // Test that all basic ASCII-range characters can be encoded/decoded
        for i in 0x20..=0x7F {
            if let Some(ch) = build_gsm_table().get(&i).and_then(|&opt_ch| opt_ch) {
                let text = ch.to_string();
                let encoded = encode(&text).unwrap();
                let decoded = decode(&encoded).unwrap();
                assert_eq!(decoded, text, "Failed for character: {} (0x{:02X})", ch, i);
            }
        }
    }

    #[test]
    fn test_all_extension_characters() {
        let ext_table = build_gsm_ext_table();
        for &ch in ext_table.values() {
            let text = ch.to_string();
            let encoded = encode(&text).unwrap();
            let decoded = decode(&encoded).unwrap();
            assert_eq!(decoded, text, "Failed for extension character: {}", ch);
        }
    }
}
