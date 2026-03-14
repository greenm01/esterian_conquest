use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    WrongSize {
        file_type: &'static str,
        expected: usize,
        actual: usize,
    },
    WrongRecordMultiple {
        file_type: &'static str,
        record_size: usize,
        actual: usize,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongSize {
                file_type,
                expected,
                actual,
            } => write!(
                f,
                "{file_type} had wrong size: expected {expected} bytes, got {actual}"
            ),
            Self::WrongRecordMultiple {
                file_type,
                record_size,
                actual,
            } => write!(
                f,
                "{file_type} had wrong size: expected a multiple of {record_size} bytes, got {actual}"
            ),
        }
    }
}

impl std::error::Error for ParseError {}

pub fn expect_size(
    data: &[u8],
    expected: usize,
    file_type: &'static str,
) -> Result<(), ParseError> {
    if data.len() == expected {
        Ok(())
    } else {
        Err(ParseError::WrongSize {
            file_type,
            expected,
            actual: data.len(),
        })
    }
}

pub fn copy_array<const N: usize>(data: &[u8]) -> [u8; N] {
    let mut out = [0u8; N];
    out.copy_from_slice(data);
    out
}

pub fn trim_ascii_field(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    text.trim_matches(char::from(0)).trim().to_string()
}

pub fn decode_real48(data: [u8; 6]) -> Option<f64> {
    let exponent = data[5];
    if exponent == 0 {
        return Some(0.0);
    }

    let sign = if (data[4] & 0x80) != 0 { -1.0 } else { 1.0 };
    let mantissa = u64::from(data[0])
        | (u64::from(data[1]) << 8)
        | (u64::from(data[2]) << 16)
        | (u64::from(data[3]) << 24)
        | (u64::from(data[4] & 0x7f) << 32);
    let fractional = mantissa as f64 / ((1u64 << 39) as f64);
    Some(sign * (1.0 + fractional) * 2f64.powi(i32::from(exponent) - 129))
}
