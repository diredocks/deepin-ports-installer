/// Print a simple aligned table with a header row.
pub fn print(headers: &[&str], rows: &[Vec<String>]) {
    let header: Vec<String> = headers.iter().map(|cell| (*cell).to_owned()).collect();
    let widths = column_widths(&header, rows);

    println!("{}", format_row(&header, &widths));
    for row in rows {
        println!("{}", format_row(row, &widths));
    }
}

fn column_widths(header: &[String], rows: &[Vec<String>]) -> Vec<usize> {
    let mut widths: Vec<usize> = header.iter().map(|cell| cell.chars().count()).collect();

    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            if let Some(width) = widths.get_mut(index) {
                *width = (*width).max(cell.chars().count());
            }
        }
    }

    widths
}

fn format_row(cells: &[String], widths: &[usize]) -> String {
    let last = cells.len().saturating_sub(1);
    let mut line = String::new();

    for (index, cell) in cells.iter().enumerate() {
        line.push_str(cell);
        if index < last {
            let padding = widths[index].saturating_sub(cell.chars().count());
            line.extend(std::iter::repeat_n(' ', padding + 2));
        }
    }

    line
}
