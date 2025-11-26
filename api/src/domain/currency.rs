//! Currencies a group can use. Amounts are always stored as an integer
//! number of the currency's minor unit (cents, pence, yen...).

/// (ISO 4217 code, number of decimal places)
const CURRENCIES: &[(&str, u8)] = &[
    ("AUD", 2),
    ("BRL", 2),
    ("CAD", 2),
    ("CHF", 2),
    ("CZK", 2),
    ("DKK", 2),
    ("EUR", 2),
    ("GBP", 2),
    ("HUF", 2),
    ("INR", 2),
    ("JPY", 0),
    ("KRW", 0),
    ("MXN", 2),
    ("NOK", 2),
    ("NZD", 2),
    ("PLN", 2),
    ("RON", 2),
    ("SEK", 2),
    ("USD", 2),
    ("ZAR", 2),
];

/// Returns the number of decimal places for a supported currency.
pub fn minor_units(code: &str) -> Option<u8> {
    CURRENCIES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, units)| *units)
}

pub fn is_supported(code: &str) -> bool {
    minor_units(code).is_some()
}

pub fn all() -> impl Iterator<Item = &'static str> {
    CURRENCIES.iter().map(|(c, _)| *c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knows_common_currencies() {
        assert_eq!(minor_units("EUR"), Some(2));
        assert_eq!(minor_units("JPY"), Some(0));
        assert!(is_supported("USD"));
    }

    #[test]
    fn rejects_unknown_and_lowercase_codes() {
        assert_eq!(minor_units("XXX"), None);
        assert_eq!(minor_units("eur"), None);
    }

    #[test]
    fn list_is_sorted_and_unique() {
        let codes: Vec<_> = all().collect();
        let mut sorted = codes.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(codes, sorted);
    }
}
