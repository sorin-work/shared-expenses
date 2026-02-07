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
