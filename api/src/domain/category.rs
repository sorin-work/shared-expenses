//! Expense categories. The list is fixed; the database has a CHECK
//! constraint with the same values.

pub const CATEGORIES: &[&str] = &[
    "general",
    "groceries",
    "rent",
    "utilities",
    "dining",
    "transport",
    "travel",
    "accommodation",
    "entertainment",
    "household",
    "gifts",
    "health",
];

pub const DEFAULT: &str = "general";

pub fn is_valid(category: &str) -> bool {
    CATEGORIES.contains(&category)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_a_valid_category() {
        assert!(is_valid(DEFAULT));
    }

    #[test]
    fn unknown_categories_are_rejected() {
        assert!(!is_valid("Groceries"));
        assert!(!is_valid(""));
        assert!(!is_valid("misc"));
    }
}
