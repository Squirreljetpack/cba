use std::{
    fmt::{Alignment, Display},
    io::{StdoutLock, Write},
};

use super::StrExt;

pub struct TableBuilder<T, U, W, const N: usize>
where
    T: Display,
    U: Display,
    W: Write,
{
    writer: W,
    header: Option<[T; N]>,
    rows: Vec<[U; N]>,
    widths: [usize; N],
    sep: Option<String>,
    header_fmter: Option<Box<dyn Fn(String, usize) -> String>>,
    cell_fmter: Option<Box<dyn Fn(String, [usize; 2]) -> String>>,
}

impl<T, U, const N: usize> TableBuilder<T, U, StdoutLock<'static>, N>
where
    T: Display,
    U: Display,
{
    pub fn new(widths: [usize; N]) -> Self {
        Self {
            writer: std::io::stdout().lock(),
            header: None,
            rows: Vec::new(),
            widths,
            sep: None,
            header_fmter: None,
            cell_fmter: None,
        }
    }
}

impl<T, U, W, const N: usize> TableBuilder<T, U, W, N>
where
    T: Display,
    U: Display,
    W: Write,
{
    pub fn new_with_writer(widths: [usize; N], writer: W) -> Self {
        Self {
            writer,
            header: None,
            rows: Vec::new(),
            widths,
            sep: None,
            header_fmter: None,
            cell_fmter: None,
        }
    }

    pub fn header(mut self, hdr: [T; N]) -> Self {
        self.header = Some(hdr);
        self
    }

    pub fn separator(mut self, sep: impl Display) -> Self {
        self.sep = Some(sep.to_string());
        self
    }

    pub fn repeated_separator(mut self, c: impl Display) -> Self {
        self.sep = Some(c.to_string().repeat(self.widths.iter().sum()));
        self
    }

    pub fn header_formatter<F>(mut self, f: F) -> Self
    where
        F: Fn(String, usize) -> String + 'static,
    {
        self.header_fmter = Some(Box::new(f));
        self
    }

    pub fn cell_formatter<F>(mut self, f: F) -> Self
    where
        F: Fn(String, [usize; 2]) -> String + 'static,
    {
        self.cell_fmter = Some(Box::new(f));
        self
    }

    pub fn row(mut self, row: [U; N]) -> Self {
        self.rows.push(row);
        self
    }

    pub fn rows(mut self, rows: impl IntoIterator<Item = [U; N]>) -> Self {
        self.rows = rows.into_iter().collect();
        self
    }

    pub fn print(&mut self) -> std::io::Result<()> {
        let w = &mut self.writer;
        // helper to align
        let align = |i: usize, val: String| {
            if i == 0 {
                val.pad_to(self.widths[i], Alignment::Left)
            } else if i == self.widths.len() - 1 {
                val.pad_to(self.widths[i], Alignment::Right)
            } else {
                val.pad_to(self.widths[i], Alignment::Center)
            }
        };

        // print header
        if let Some(hdr) = self.header.as_ref() {
            let mut s = String::new();
            for (i, val) in hdr.into_iter().enumerate() {
                let mut aligned = align(i, val.to_string());
                if let Some(ref f) = self.header_fmter {
                    aligned = f(aligned, i);
                }
                s.push_str(&aligned);
            }
            writeln!(w, "{s}")?;
        }

        // separator
        if let Some(sep) = self.sep.as_ref() {
            writeln!(w, "{sep}")?;
        }

        // print rows
        for (r_idx, row) in self.rows.iter().enumerate() {
            let mut s = String::new();
            for (i, val) in row.into_iter().enumerate() {
                let mut aligned = align(i, val.to_string());
                if let Some(ref f) = self.cell_fmter {
                    aligned = f(aligned, [r_idx, i]);
                }
                s.push_str(&aligned);
            }
            writeln!(w, "{s}")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_write_justified_table() {
        let buf: Vec<u8> = Vec::new();
        let mut cursor = Cursor::new(buf);

        let mut table = TableBuilder::new_with_writer([11, 10, 9], &mut cursor)
            .header(["chars", "words", "lines"])
            .repeated_separator('-')
            .rows([[1234, 210, 42], [56789, 1234, 100]]);

        table.print().unwrap();

        let expected = "\
chars         words      lines
------------------------------
1234           210          42
56789         1234         100
";

        assert_eq!(
            String::from_utf8(cursor.into_inner()).expect("valid UTF-8"),
            expected
        );
    }
}
