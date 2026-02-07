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

#[cfg(test)]
mod tests {
    use super::*;

    fn line(fields: &[&str]) -> String {
        let mut w = CsvWriter::new();
        w.row(fields);
        w.finish()
    }

    #[test]
    fn writes_plain_fields() {
        assert_eq!(line(&["a", "b c", "12.50"]), "a,b c,12.50\r\n");
    }

    #[test]
    fn quotes_fields_with_separators_and_quotes() {
        assert_eq!(
            line(&["a,b", "say \"hi\"", "two\nlines"]),
            "\"a,b\",\"say \"\"hi\"\"\",\"two\nlines\"\r\n"
        );
    }

    #[test]
    fn neutralises_formulas() {
        assert_eq!(
            line(&["=SUM(A1)", "@cmd", "+1"]),
            "\"'=SUM(A1)\",\"'@cmd\",\"'+1\"\r\n"
        );
        assert_eq!(line(&["-HYPERLINK()"]), "\"'-HYPERLINK()\"\r\n");
    }

    #[test]
    fn empty_fields_stay_empty() {
        assert_eq!(line(&["", "x", ""]), ",x,\r\n");
    }
}
