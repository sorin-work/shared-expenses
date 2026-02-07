//! A tiny CSV writer (RFC 4180 quoting), enough for the export.

#[derive(Default)]
pub struct CsvWriter {
    out: String,
}

impl CsvWriter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn row<I, S>(&mut self, fields: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut first = true;
        for field in fields {
            if !first {
                self.out.push(',');
            }
            first = false;
            push_field(&mut self.out, field.as_ref());
        }
        self.out.push_str("\r\n");
    }

    pub fn finish(self) -> String {
        self.out
    }
}

fn push_field(out: &mut String, value: &str) {
    // Spreadsheet apps run cells starting with these as formulas.
    let formula = value.starts_with(['=', '+', '-', '@', '\t', '\r']);
    let needs_quotes = formula || value.contains([',', '"', '\n', '\r']);

    if !needs_quotes {
        out.push_str(value);
        return;
    }
    out.push('"');
    if formula {
        out.push('\'');
    }
    out.push_str(&value.replace('"', "\"\""));
    out.push('"');
}
