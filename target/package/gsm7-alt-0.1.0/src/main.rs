use gsm7::{decode, encode, encoded_len, is_gsm7_compatible, Gsm7Config};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let text = "Hello {world} €!";

    println!("Original: {}", text);
    println!("GSM 7-bit compatible: {}", is_gsm7_compatible(text));
    println!("Encoded length: {} bytes", encoded_len(text)?);

    let encoded = encode(text)?;
    println!("Encoded: {:?}", encoded);

    let decoded = decode(&encoded)?;
    println!("Decoded: {}", decoded);

    // Example with non-strict mode
    let emoji_text = "Hello 🦀 World";
    let config = Gsm7Config {
        strict: false,
        replacement_char: '?',
    };

    let encoded_with_replacement = gsm7::encode_with_config(emoji_text, &config)?;
    let decoded_with_replacement = decode(&encoded_with_replacement)?;
    println!(
        "With replacement: {} -> {}",
        emoji_text, decoded_with_replacement
    );

    Ok(())
}
