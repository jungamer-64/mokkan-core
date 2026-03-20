use crate::application::{AppResult, error::AppError};

/// Generate a lowercase hyphenated RFC 4122 version 4 identifier string.
///
/// # Errors
///
/// Returns an error if the operating system random source cannot provide
/// enough entropy for ID generation.
pub fn v4_string() -> AppResult<String> {
    let mut bytes = [0_u8; 16];
    getrandom::getrandom(&mut bytes)
        .map_err(|err| AppError::infrastructure(format!("failed to generate random id: {err}")))?;

    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    Ok(encode_hyphenated_lowercase(&bytes))
}

fn encode_hyphenated_lowercase(bytes: &[u8; 16]) -> String {
    let mut value = String::with_capacity(36);

    for (index, byte) in bytes.iter().copied().enumerate() {
        if matches!(index, 4 | 6 | 8 | 10) {
            value.push('-');
        }
        push_hex_byte(&mut value, byte);
    }

    value
}

fn push_hex_byte(output: &mut String, byte: u8) {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    output.push(char::from(HEX[usize::from(byte >> 4)]));
    output.push(char::from(HEX[usize::from(byte & 0x0f)]));
}

#[cfg(test)]
mod tests {
    use super::v4_string;
    use std::collections::HashSet;

    #[test]
    fn generates_lowercase_hyphenated_v4_strings() {
        let value = v4_string().expect("random id");
        let bytes = value.as_bytes();

        assert_eq!(value.len(), 36);
        assert_eq!(bytes[8], b'-');
        assert_eq!(bytes[13], b'-');
        assert_eq!(bytes[18], b'-');
        assert_eq!(bytes[23], b'-');
        assert_eq!(bytes[14], b'4');
        assert!(matches!(bytes[19], b'8' | b'9' | b'a' | b'b'));
        assert!(
            value
                .chars()
                .all(|c| matches!(c, '0'..='9' | 'a'..='f' | '-'))
        );
    }

    #[test]
    fn generates_distinct_values_in_smoke_test() {
        let mut values = HashSet::new();

        for _ in 0..128 {
            let value = v4_string().expect("random id");
            assert!(values.insert(value));
        }
    }
}
