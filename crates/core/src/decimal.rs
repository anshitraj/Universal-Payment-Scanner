use crate::{ErrorCode, ParseError};

pub fn validate_decimal(value: &str, max_scale: usize) -> Result<String, ParseError> {
    if value.is_empty() || value.len() > 128 || value.starts_with('-') || value.starts_with('+') {
        return Err(ParseError::new(
            ErrorCode::InvalidAmount,
            "Amount must be a positive decimal.",
        ));
    }
    let mut dot = false;
    let mut scale = 0usize;
    for byte in value.bytes() {
        match byte {
            b'0'..=b'9' if dot => scale += 1,
            b'0'..=b'9' => {}
            b'.' if !dot => dot = true,
            _ => {
                return Err(ParseError::new(
                    ErrorCode::InvalidAmount,
                    "Amount contains invalid characters.",
                ));
            }
        }
    }
    if value == "." || value.ends_with('.') || scale > max_scale {
        return Err(ParseError::new(
            ErrorCode::InvalidAmount,
            "Amount has an invalid decimal scale.",
        ));
    }
    let trimmed = value.trim_start_matches('0');
    let normalized = if trimmed.is_empty() || trimmed.starts_with('.') {
        format!("0{trimmed}")
    } else {
        trimmed.to_owned()
    };
    Ok(normalized)
}

pub fn integer_scientific_to_decimal(value: &str, scale: usize) -> Result<String, ParseError> {
    if value.len() > 128 || value.starts_with('-') || value.starts_with('+') {
        return Err(ParseError::new(
            ErrorCode::InvalidAmount,
            "Atomic amount must be non-negative.",
        ));
    }
    let lower = value.to_ascii_lowercase();
    let (coefficient, exponent) = lower.split_once('e').unwrap_or((&lower, "0"));
    let exponent: i32 = exponent
        .parse()
        .map_err(|_| ParseError::new(ErrorCode::InvalidAmount, "Invalid amount exponent."))?;
    let (whole, fraction) = coefficient.split_once('.').unwrap_or((coefficient, ""));
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(ParseError::new(
            ErrorCode::InvalidAmount,
            "Invalid numeric amount.",
        ));
    }
    let mut digits = format!("{whole}{fraction}");
    let decimal_shift = exponent - fraction.len() as i32;
    if decimal_shift < 0 {
        return Err(ParseError::new(
            ErrorCode::InvalidAmount,
            "Atomic amount is not an integer.",
        ));
    }
    if digits.len() + decimal_shift as usize > 128 {
        return Err(ParseError::new(
            ErrorCode::InvalidAmount,
            "Amount exceeds parser limits.",
        ));
    }
    digits.extend(std::iter::repeat_n('0', decimal_shift as usize));
    let digits = digits.trim_start_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    if scale == 0 {
        return Ok(digits.into());
    }
    if digits.len() <= scale {
        return Ok(format!("0.{}{digits}", "0".repeat(scale - digits.len()))
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned());
    }
    let split = digits.len() - scale;
    Ok(format!("{}.{}", &digits[..split], &digits[split..])
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_atomic_scientific_notation_without_floats() {
        assert_eq!(
            integer_scientific_to_decimal("2.014e18", 18).unwrap(),
            "2.014"
        );
        assert_eq!(
            integer_scientific_to_decimal("1000000000000000000", 18).unwrap(),
            "1"
        );
        assert!(integer_scientific_to_decimal("1.2", 18).is_err());
    }
}
