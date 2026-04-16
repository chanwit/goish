//! text/tabwriter: column-aligned output.
//!
//!   Go                                  goish
//!   ─────────────────────────────────   ──────────────────────────────────
//!   tw := tabwriter.NewWriter(w, 0,     let mut tw = tabwriter::NewWriter(
//!     8, 1, ' ', 0)                        0, 8, 1, ' ', 0);
//!   fmt.Fprintln(tw, "a\tbb\tccc")      tw.Write(b"a\tbb\tccc\n");
//!   tw.Flush()                          let out = tw.Flush();
//!
//! Differences from Go: goish's Writer is self-contained — it buffers
//! input in-memory; `Flush()` returns the aligned output as a `String`
//! instead of writing to an underlying `io.Writer`. This matches goish's
//! pattern for simple text transforms (`strings::Builder`, `csv::Writer`).
//! Flags (AlignRight, FilterHTML, etc.) are not implemented.

use crate::types::int;

/// Tabwriter that buffers tab-separated input and aligns columns.
pub struct Writer {
    minwidth: usize,
    tabwidth: usize,
    padding: usize,
    padchar: char,
    buf: String,
}

/// `tabwriter.NewWriter(minwidth, tabwidth, padding, padchar, flags)` —
/// Go's first arg is an `io.Writer`; goish omits it (buffered output is
/// returned by Flush).
///
/// `flags` is currently ignored.
#[allow(non_snake_case)]
pub fn NewWriter(minwidth: int, tabwidth: int, padding: int, padchar: char, _flags: int) -> Writer {
    Writer {
        minwidth: minwidth.max(0) as usize,
        tabwidth: tabwidth.max(1) as usize,
        padding: padding.max(0) as usize,
        padchar,
        buf: String::new(),
    }
}

impl Writer {
    pub fn Write(&mut self, p: &[u8]) -> (int, crate::errors::error) {
        self.buf.push_str(std::str::from_utf8(p).unwrap_or(""));
        (p.len() as int, crate::errors::nil)
    }

    /// Write a string slice directly. Convenience, not in Go.
    pub fn WriteString(&mut self, s: &str) -> (int, crate::errors::error) {
        self.buf.push_str(s);
        (s.len() as int, crate::errors::nil)
    }

    /// `tw.Flush()` — compute column widths across all buffered rows,
    /// pad each cell, and return the aligned output. Clears the buffer.
    pub fn Flush(&mut self) -> String {
        let input = std::mem::take(&mut self.buf);
        // Split into lines, keeping trailing newline behavior.
        let mut rows: Vec<Vec<&str>> = Vec::new();
        for line in input.split_inclusive('\n') {
            // Strip trailing newline if present.
            let trimmed = line.strip_suffix('\n').unwrap_or(line);
            let cells: Vec<&str> = trimmed.split('\t').collect();
            rows.push(cells);
        }
        // Determine max column count and per-column widths.
        let max_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
        let mut widths: Vec<usize> = vec![0; max_cols];
        for row in &rows {
            // The last cell is unpadded (post-tab trailer).
            for i in 0..row.len().saturating_sub(1) {
                let w = row[i].chars().count();
                if w > widths[i] { widths[i] = w; }
            }
        }
        // Apply minwidth and padding.
        for w in widths.iter_mut() {
            if *w < self.minwidth { *w = self.minwidth; }
        }
        // Emit.
        let mut out = String::with_capacity(input.len() + max_cols * 4);
        for row in &rows {
            for (i, cell) in row.iter().enumerate() {
                out.push_str(cell);
                if i + 1 < row.len() {
                    // Pad to width + padding.
                    let cell_width = cell.chars().count();
                    let target = widths[i] + self.padding;
                    let pad_n = target.saturating_sub(cell_width);
                    for _ in 0..pad_n { out.push(self.padchar); }
                }
            }
            out.push('\n');
        }
        // Respect the original absence of a trailing newline.
        if !input.ends_with('\n') && out.ends_with('\n') {
            out.pop();
        }
        // Reference the field once to quiet the unused warning in builds
        // where no padding is used.
        let _ = self.tabwidth;
        out
    }

    /// Return the buffered (not-yet-flushed) input.
    pub fn Buffered(&self) -> &str { &self.buf }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aligns_two_columns() {
        let mut tw = NewWriter(0, 8, 1, ' ', 0);
        tw.WriteString("a\t1\nbb\t22\nccc\t333\n");
        let out = tw.Flush();
        // widths[0] = 3 ("ccc"), padding = 1, so first col is 4 chars.
        assert_eq!(out, "a   1\nbb  22\nccc 333\n");
    }

    #[test]
    fn single_column_is_passthrough() {
        let mut tw = NewWriter(0, 8, 1, ' ', 0);
        tw.WriteString("hello\nworld\n");
        assert_eq!(tw.Flush(), "hello\nworld\n");
    }

    #[test]
    fn minwidth_applies() {
        let mut tw = NewWriter(5, 8, 0, '.', 0);
        tw.WriteString("a\t1\n");
        assert_eq!(tw.Flush(), "a....1\n");
    }
}
