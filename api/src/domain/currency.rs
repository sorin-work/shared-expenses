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
