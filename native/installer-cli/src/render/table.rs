use comfy_table::{Cell, Table, presets};

/// Print a Markdown-style table with a header row.
pub fn print(headers: &[&str], rows: &[Vec<String>]) {
    let mut table = Table::new();
    table.load_preset(presets::ASCII_MARKDOWN);

    table.set_header(headers.iter().map(Cell::new));
    for row in rows {
        table.add_row(row.iter().map(Cell::new));
    }

    println!("{}", table.trim_fmt());
}
