//! Formatting of minor-unit amounts, used by the CSV export.

/// Formats an amount in minor units as a plain decimal string, e.g.
/// `-1234` with 2 decimals becomes `"-12.34"`. No thousands separators and
/// no currency symbol, so spreadsheets can parse it.
pub fn format_minor(amount: i64, decimals: u8) -> String {
    if decimals == 0 {
        return amount.to_string();
    }
    let sign = if amount < 0 { "-" } else { "" };
    let abs = amount.unsigned_abs();
    let factor = 10u64.pow(u32::from(decimals));
    format!(
        "{sign}{}.{:0width$}",
        abs / factor,
        abs % factor,
        width = usize::from(decimals)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_two_decimal_amounts() {
        assert_eq!(format_minor(1234, 2), "12.34");
        assert_eq!(format_minor(5, 2), "0.05");
        assert_eq!(format_minor(100, 2), "1.00");
        assert_eq!(format_minor(0, 2), "0.00");
    }

    #[test]
    fn formats_negative_amounts() {
        assert_eq!(format_minor(-1234, 2), "-12.34");
        assert_eq!(format_minor(-5, 2), "-0.05");
    }

    #[test]
    fn formats_zero_decimal_currencies() {
        assert_eq!(format_minor(1500, 0), "1500");
        assert_eq!(format_minor(-7, 0), "-7");
    }

    #[test]
    fn handles_extreme_values() {
        assert_eq!(format_minor(i64::MIN, 2), "-92233720368547758.08");
    }
}
