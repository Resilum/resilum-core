const BETWEEN_COLUMNS: &str = "  ";

pub struct Cell {
    plain: String,
    shown: String,
    to_the_right: bool,
}

impl Cell {
    pub fn plain(text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            plain: text.clone(),
            shown: text,
            to_the_right: false,
        }
    }

    pub fn painted(plain: impl Into<String>, shown: impl Into<String>) -> Self {
        Self {
            plain: plain.into(),
            shown: shown.into(),
            to_the_right: false,
        }
    }

    pub fn leaning_right(mut self) -> Self {
        self.to_the_right = true;
        self
    }

    fn width(&self) -> usize {
        self.plain.chars().count()
    }

    fn padded(&self, column: usize) -> String {
        let room = " ".repeat(column.saturating_sub(self.width()));
        if self.to_the_right {
            format!("{room}{}", self.shown)
        } else {
            format!("{}{room}", self.shown)
        }
    }
}

pub fn laid_out(rows: &[Vec<Cell>]) -> String {
    let widest = widths(rows);
    rows.iter()
        .map(|row| one_row(row, &widest))
        .collect::<Vec<_>>()
        .join("\n")
}

fn widths(rows: &[Vec<Cell>]) -> Vec<usize> {
    let mut widest = vec![0; rows.iter().map(Vec::len).max().unwrap_or(0)];
    for row in rows {
        for (nth, cell) in row.iter().enumerate() {
            widest[nth] = widest[nth].max(cell.width());
        }
    }
    widest
}

fn one_row(row: &[Cell], widest: &[usize]) -> String {
    let last = row.len().saturating_sub(1);
    let line: String = row
        .iter()
        .enumerate()
        .map(|(nth, cell)| {
            let column = if nth == last { 0 } else { widest[nth] };
            format!("{}{BETWEEN_COLUMNS}", cell.padded(column))
        })
        .collect();
    line.trim_end().to_owned()
}

#[cfg(test)]
#[path = "grid_tests.rs"]
mod tests;
