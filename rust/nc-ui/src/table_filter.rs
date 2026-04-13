#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKind {
    Text,
    Number,
    Coord,
    Bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableFilterColumn {
    pub code: &'static str,
    pub label: &'static str,
    pub aliases: &'static [&'static str],
    pub kind: FilterKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberOp {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
}

impl NumberOp {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Ne => "!=",
            Self::Gt => ">",
            Self::Gte => ">=",
            Self::Lt => "<",
            Self::Lte => "<=",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableFilterPredicate {
    TextContains(String),
    Number { op: NumberOp, value: i64 },
    CoordExact([u8; 2]),
    CoordRadius { anchor: [u8; 2], radius: u8 },
    Bool(bool),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableFilterClause {
    pub column: TableFilterColumn,
    pub predicate: TableFilterPredicate,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColumnCodeParseError {
    Unknown,
    Ambiguous(Vec<&'static str>),
}

pub fn parse_column_code(
    columns: &'static [TableFilterColumn],
    input: &str,
) -> Result<TableFilterColumn, ColumnCodeParseError> {
    let raw = input.trim().to_ascii_lowercase();
    if let Some(column) = columns.iter().copied().find(|column| column.code == raw) {
        return Ok(column);
    }
    let exact_matches = columns
        .iter()
        .copied()
        .filter(|column| column_matches_exact(column, &raw))
        .collect::<Vec<_>>();
    match exact_matches.as_slice() {
        [] => {}
        [column] => return Ok(*column),
        _ => {
            return Err(ColumnCodeParseError::Ambiguous(dedupe_codes(
                &exact_matches,
            )));
        }
    }
    let matches = columns
        .iter()
        .copied()
        .filter(|column| column_matches_prefix(column, &raw))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Err(ColumnCodeParseError::Unknown),
        [column] => Ok(*column),
        _ => {
            let codes = dedupe_codes(&matches);
            if codes.len() == 1 {
                Ok(matches[0])
            } else {
                Err(ColumnCodeParseError::Ambiguous(codes))
            }
        }
    }
}

pub fn format_column_code_error(error: &ColumnCodeParseError) -> String {
    match error {
        ColumnCodeParseError::Unknown => "Enter a valid column name/code or ALL.".to_string(),
        ColumnCodeParseError::Ambiguous(codes) => {
            format!("Ambiguous: {}", codes.join("/"))
        }
    }
}

pub fn parse_filter_clause(
    column: TableFilterColumn,
    input: &str,
) -> Result<TableFilterClause, String> {
    let raw = input.trim();
    let predicate = match column.kind {
        FilterKind::Text => parse_text_predicate(raw)?,
        FilterKind::Number => parse_number_predicate(raw)?,
        FilterKind::Coord => parse_coord_predicate(raw)?,
        FilterKind::Bool => parse_bool_predicate(raw)?,
    };
    Ok(TableFilterClause {
        column,
        summary: summarize_clause(column.code, &predicate),
        predicate,
    })
}

pub fn is_filter_column_char(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == ' '
}

pub fn is_filter_value_char(kind: FilterKind, ch: char) -> bool {
    match kind {
        FilterKind::Text => ch.is_ascii_alphanumeric() || matches!(ch, ' ' | '-' | '#' | '*' | '/'),
        FilterKind::Number => {
            ch.is_ascii_digit() || matches!(ch, '?' | '=' | '!' | '>' | '<' | '+' | '-')
        }
        FilterKind::Coord => {
            ch.is_ascii_digit() || matches!(ch, ',' | '/' | ' ' | '(' | ')' | '[' | ']' | '{' | '}')
        }
        FilterKind::Bool => ch.is_ascii_alphanumeric(),
    }
}

impl TableFilterPredicate {
    pub fn matches_text(&self, value: Option<&str>) -> bool {
        match self {
            Self::TextContains(needle) => value
                .map(|value| value.to_ascii_lowercase().contains(needle))
                .unwrap_or(false),
            Self::Unknown => value.is_none(),
            _ => false,
        }
    }

    pub fn matches_number(&self, value: Option<i64>) -> bool {
        match self {
            Self::Number {
                op,
                value: expected,
            } => value
                .map(|actual| match op {
                    NumberOp::Eq => actual == *expected,
                    NumberOp::Ne => actual != *expected,
                    NumberOp::Gt => actual > *expected,
                    NumberOp::Gte => actual >= *expected,
                    NumberOp::Lt => actual < *expected,
                    NumberOp::Lte => actual <= *expected,
                })
                .unwrap_or(false),
            Self::Unknown => value.is_none(),
            _ => false,
        }
    }

    pub fn matches_coord(&self, value: [u8; 2]) -> bool {
        match self {
            Self::CoordExact(expected) => &value == expected,
            Self::CoordRadius { anchor, radius } => {
                let dx = i32::from(anchor[0]) - i32::from(value[0]);
                let dy = i32::from(anchor[1]) - i32::from(value[1]);
                let distance_sq = (dx * dx + dy * dy) as u32;
                distance_sq <= u32::from(*radius) * u32::from(*radius)
            }
            _ => false,
        }
    }

    pub fn matches_bool(&self, value: bool) -> bool {
        match self {
            Self::Bool(expected) => *expected == value,
            _ => false,
        }
    }
}

fn parse_text_predicate(raw: &str) -> Result<TableFilterPredicate, String> {
    if raw.is_empty() {
        return Err("Enter filter text.".to_string());
    }
    if raw == "?" {
        return Ok(TableFilterPredicate::Unknown);
    }
    Ok(TableFilterPredicate::TextContains(raw.to_ascii_lowercase()))
}

fn parse_number_predicate(raw: &str) -> Result<TableFilterPredicate, String> {
    if raw.is_empty() {
        return Err("Enter a number or comparison.".to_string());
    }
    if raw == "?" {
        return Ok(TableFilterPredicate::Unknown);
    }
    let (op, number) = if let Some(rest) = raw.strip_prefix(">=") {
        (NumberOp::Gte, rest)
    } else if let Some(rest) = raw.strip_prefix("<=") {
        (NumberOp::Lte, rest)
    } else if let Some(rest) = raw.strip_prefix("!=") {
        (NumberOp::Ne, rest)
    } else if let Some(rest) = raw.strip_prefix('>') {
        (NumberOp::Gt, rest)
    } else if let Some(rest) = raw.strip_prefix('<') {
        (NumberOp::Lt, rest)
    } else if let Some(rest) = raw.strip_prefix('=') {
        (NumberOp::Eq, rest)
    } else {
        (NumberOp::Eq, raw)
    };
    let value = number
        .trim()
        .parse::<i64>()
        .map_err(|_| "Enter a valid number or comparison.".to_string())?;
    Ok(TableFilterPredicate::Number { op, value })
}

fn parse_coord_predicate(raw: &str) -> Result<TableFilterPredicate, String> {
    if raw.is_empty() {
        return Err("Enter coordinates like 12,7 or 12,7/5.".to_string());
    }
    let (coord_part, radius_part) = if let Some((coords, radius)) = raw.split_once('/') {
        (coords, Some(radius))
    } else {
        (raw, None)
    };
    let anchor = parse_coords(coord_part)
        .ok_or_else(|| "Enter coordinates like 12,7 or 12,7/5.".to_string())?;
    if let Some(radius) = radius_part {
        let radius = radius
            .trim()
            .parse::<u8>()
            .map_err(|_| "Enter a range radius from 0 to 255.".to_string())?;
        return Ok(TableFilterPredicate::CoordRadius { anchor, radius });
    }
    Ok(TableFilterPredicate::CoordExact(anchor))
}

fn parse_bool_predicate(raw: &str) -> Result<TableFilterPredicate, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" | "selected" | "x" | "true" | "1" => Ok(TableFilterPredicate::Bool(true)),
        "n" | "no" | "unselected" | "blank" | "false" | "0" => {
            Ok(TableFilterPredicate::Bool(false))
        }
        _ => Err("Enter yes or no.".to_string()),
    }
}

fn parse_coords(raw: &str) -> Option<[u8; 2]> {
    let digits = raw
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let [x, y] = digits.as_slice() else {
        return None;
    };
    Some([x.parse().ok()?, y.parse().ok()?])
}

fn summarize_clause(code: &str, predicate: &TableFilterPredicate) -> String {
    let code = code.to_ascii_uppercase();
    match predicate {
        TableFilterPredicate::TextContains(value) => format!("{code}~{value}"),
        TableFilterPredicate::Number { op, value } => format!("{code}{}{value}", op.label()),
        TableFilterPredicate::CoordExact([x, y]) => format!("{code}={x},{y}"),
        TableFilterPredicate::CoordRadius {
            anchor: [x, y],
            radius,
        } => format!("{code}={x},{y}/{radius}"),
        TableFilterPredicate::Bool(true) => format!("{code}=Y"),
        TableFilterPredicate::Bool(false) => format!("{code}=N"),
        TableFilterPredicate::Unknown => format!("{code}=?"),
    }
}

fn column_matches_exact(column: &TableFilterColumn, raw: &str) -> bool {
    let normalized = normalize_match_key(raw);
    !normalized.is_empty()
        && column_match_tokens(column)
            .into_iter()
            .any(|token| token == normalized)
}

fn column_matches_prefix(column: &TableFilterColumn, raw: &str) -> bool {
    let normalized = normalize_match_key(raw);
    !normalized.is_empty()
        && column_match_tokens(column)
            .into_iter()
            .any(|token| token.starts_with(&normalized))
}

fn column_match_tokens(column: &TableFilterColumn) -> Vec<String> {
    let mut tokens = vec![
        normalize_match_key(column.code),
        normalize_match_key(column.label),
    ];
    tokens.extend(split_column_tokens(column.label));
    for alias in column.aliases {
        tokens.push(normalize_match_key(alias));
        tokens.extend(split_column_tokens(alias));
    }
    tokens.retain(|token| !token.is_empty());
    tokens.sort();
    tokens.dedup();
    tokens
}

fn split_column_tokens(label: &str) -> Vec<String> {
    label
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(normalize_match_key)
        .collect()
}

fn dedupe_codes(columns: &[TableFilterColumn]) -> Vec<&'static str> {
    let mut codes = columns.iter().map(|column| column.code).collect::<Vec<_>>();
    codes.sort_unstable();
    codes.dedup();
    codes
}

fn normalize_match_key(raw: &str) -> String {
    raw.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}
