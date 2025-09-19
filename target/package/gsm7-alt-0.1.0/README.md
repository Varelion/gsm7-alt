# gsm7-alt
This crate is really a “GSM 7-bit character set mapper”, not a full “GSM 7-bit encoder” in the SMS sense.

This crate provides:

Mapping between Unicode text and the GSM 7-bit character set (base + extension table).

Configurable handling of unsupported characters.

Utilities for checking compatibility and estimating length.

⚠️ Important:
This crate currently maps characters to GSM 7-bit codes but does not perform septet packing (the bit-level compression used in SMS PDUs). If you need to generate raw SMS payloads, you must add an additional packing step.


Example usage:
let decoded = gsm7_alt::decode(&data).map_err(|e| e.to_string())?;

use gsm7_alt::{encode, decode};

fn main() -> Result<(), Box<dyn std::error::Error>> {
let text = "Hello {world} €!";

    let encoded = encode(text)?;
    let decoded = decode(&encoded)?;

    assert_eq!(decoded, text);
    println!("Decoded text: {}", decoded);

    Ok(())

}
