// Encode a string using GSM 7-bit encoding with custom configuration.
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
    if content.is_empty() {
        return Ok(Vec::new());
    }

    // Check length limit
    if config.max_input_length > 0 && content.len() > config.max_input_length {
        return Err(Gsm7Error::malformed_data(format!(
            "Input length {} exceeds maximum of {}",
            content.len(),
            config.max_input_length
        )));
    }

    let maps = &*GSM_MAPS;
    let mut bytes = Vec::with_capacity(content.len());

    for ch in content.chars() {
        match maps.get_code(ch) {
            Some(Code::Single(b)) => bytes.push(*b),
            Some(Code::Escape(b)) => {
                bytes.push(0x1B);
                bytes.push(*b);
            }
            None => {
                if config.strict {
                    return Err(Gsm7Error::unsupported_character(ch));
                } else {
                    // Use replacement character
                    match maps.get_code(config.replacement_char) {
                        Some(Code::Single(b)) => bytes.push(*b),
                        Some(Code::Escape(b)) => {
                            bytes.push(0x1B);
                            bytes.push(*b);
                        }
                        None => bytes.push(0x20), // Space as ultimate fallback
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
/// A `String` containing the decoded text.
///
/// # Errors
///
/// Returns `Gsm7Error::InvalidByte` or `Gsm7Error::InvalidEscapeSequence`
/// if the input contains invalid GSM 7-bit data.
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
    if data.is_empty() {
        return Ok(String::new());
    }

    // Check length limit
    if config.max_input_length > 0 && data.len() > config.max_input_length {
        return Err(Gsm7Error::malformed_data(format!(
            "Input length {} exceeds maximum of {}",
            data.len(),
            config.max_input_length
        )));
    }

    if config.validate_input {
        validate_encoded_data(data, config.strict)?;
    }

    let maps = &*GSM_MAPS;
    let mut result = String::with_capacity(data.len());

    let mut i = 0;
    while i < data.len() {
        let code = data[i];

        if code == 0x1B {
            // Handle escape sequence
            if let Some(&next_code) = data.get(i + 1) {
                match maps.get_ext_char(next_code) {
                    Some(ch) => {
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
                    return Err(Gsm7Error::malformed_data("Escape byte at end of input"));
                } else {
                    result.push(config.replacement_char);
                    i += 1;
                    continue;
                }
            }
        } else {
            // Handle regular character
            match maps.get_char(code) {
                Some(ch) => result.push(ch),
                None => {
                    if config.strict {
                        return Err(Gsm7Error::InvalidByte { byte: code });
                    } else {
                        result.push(config.replacement_char);
                    }
                }
            }
        }
        i += 1;
    }

    Ok(result)
}

/// Validate encoded GSM 7-bit data.
fn validate_encoded_data(data: &[u8], strict: bool) -> Result<()> {
    let maps = &*GSM_MAPS;

    let mut i = 0;
    while i < data.len() {
        let code = data[i];

        if code == 0x1B {
            // Validate escape sequence
            match data.get(i + 1) {
                Some(&next_code) => {
                    if strict && maps.get_ext_char(next_code).is_none() {
                        return Err(Gsm7Error::InvalidEscapeSequence { code: next_code });
                    }
                    i += 2;
                }
                None => {
                    return Err(Gsm7Error::malformed_data("Escape byte at end of input"));
                }
            }
        } else if code >= 128 {
            // Invalid byte (GSM 7-bit should not have bytes >= 128)
            if strict {
                return Err(Gsm7Error::InvalidByte { byte: code });
            }
            i += 1;
        } else {
            // Valid range, check if character exists
            if strict && maps.get_char(code).is_none() {
                return Err(Gsm7Error::InvalidByte { byte: code });
            }
            i += 1;
        }
    }

    Ok(())
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
    fn test_empty_input() {
        assert!(encode("").unwrap().is_empty());
        assert!(decode(&[]).unwrap().is_empty());
    }

    #[test]
    fn test_unsupported_character_strict() {
        let result = encode("Hello 🦀 World");
        assert!(matches!(
            result,
            Err(Gsm7Error::UnsupportedCharacter { .. })
        ));
    }

    #[test]
    fn test_unsupported_character_lenient() {
        let config = Gsm7Config::lenient();
        let encoded = encode_with_config("Hello 🦀 World", &config).unwrap();
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, "Hello ? World");
    }

    #[test]
    fn test_length_limits() {
        let config = Gsm7Config::default().with_max_length(5);
        let result = encode_with_config("Hello World", &config);
        assert!(matches!(result, Err(Gsm7Error::MalformedData { .. })));
    }

    #[test]
    fn test_invalid_escape_sequence() {
        let invalid_data = [0x1B, 0xFF];
        let result = decode(&invalid_data);
        assert!(matches!(
            result,
            Err(Gsm7Error::InvalidEscapeSequence { .. })
        ));
    }
}
