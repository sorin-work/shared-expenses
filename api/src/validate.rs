//! Small helpers for collecting validation errors per field.

use crate::error::{AppError, FieldErrors};

#[derive(Default)]
pub struct Validator {
    errors: FieldErrors,
}

impl Validator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, field: &str, message: impl Into<String>) {
        self.errors
            .entry(field.to_string())
            .or_insert_with(|| message.into());
    }

    pub fn check(&mut self, ok: bool, field: &str, message: &str) {
        if !ok {
            self.add(field, message);
        }
    }

    /// Trims `value` and checks it is between 1 and `max` characters.
    pub fn text(&mut self, field: &str, value: &str, max: usize) -> String {
        let value = value.trim();
        if value.is_empty() {
            self.add(field, "is required");
        } else if value.chars().count() > max {
            self.add(field, format!("must be at most {max} characters"));
        }
        value.to_string()
    }

    /// Like [`Validator::text`] but an empty value becomes `None`.
    pub fn optional_text(
        &mut self,
        field: &str,
        value: Option<&str>,
        max: usize,
    ) -> Option<String> {
        let value = value.map(str::trim).filter(|v| !v.is_empty())?;
        if value.chars().count() > max {
            self.add(field, format!("must be at most {max} characters"));
        }
        Some(value.to_string())
    }

    pub fn email(&mut self, field: &str, value: &str) -> String {
        let value = value.trim().to_string();
        if !is_email(&value) {
            self.add(field, "is not a valid email address");
        }
        value
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn finish(self) -> Result<(), AppError> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(AppError::Validation(self.errors))
        }
    }
}

/// Deliberately loose: something@something.tld without spaces.
fn is_email(value: &str) -> bool {
    if value.len() > 254 || value.chars().any(char::is_whitespace) {
        return false;
    }
    let Some((local, domain)) = value.rsplit_once('@') else {
        return false;
    };
    !local.is_empty()
        && !local.contains('@')
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_ordinary_emails() {
        for email in ["a@b.co", "first.last+tag@example.com", "x@sub.domain.org"] {
            assert!(is_email(email), "{email}");
        }
    }

    #[test]
    fn rejects_malformed_emails() {
        for email in [
            "",
            "plain",
            "@example.com",
            "a@b",
            "a@.com",
            "a@com.",
            "a b@c.de",
            "a@b@c.de",
        ] {
            assert!(!is_email(email), "{email}");
        }
    }

    #[test]
    fn text_is_trimmed_and_checked() {
        let mut v = Validator::new();
        assert_eq!(v.text("name", "  Flat 4B  ", 10), "Flat 4B");
        assert!(!v.has_errors());
        v.text("empty", "   ", 10);
        v.text("long", "abcdefghijk", 10);
        let Err(AppError::Validation(fields)) = v.finish() else {
            panic!("expected validation errors");
        };
        assert_eq!(fields["empty"], "is required");
        assert_eq!(fields["long"], "must be at most 10 characters");
    }

    #[test]
    fn length_counts_characters_not_bytes() {
        let mut v = Validator::new();
        v.text("name", "Café Zürich", 11);
        assert!(!v.has_errors());
    }

    #[test]
    fn optional_text_turns_blank_into_none() {
        let mut v = Validator::new();
        assert_eq!(v.optional_text("notes", Some("  "), 5), None);
        assert_eq!(v.optional_text("notes", None, 5), None);
        assert_eq!(v.optional_text("notes", Some(" hi "), 5), Some("hi".into()));
        assert!(!v.has_errors());
    }
}
