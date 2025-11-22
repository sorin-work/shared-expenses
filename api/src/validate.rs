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
