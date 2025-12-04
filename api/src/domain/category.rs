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
