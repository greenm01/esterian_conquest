use nc_ui::table_filter::{
    FilterKind, NumberOp, TableFilterColumn, TableFilterPredicate, is_filter_column_char,
    is_filter_value_char, parse_column_code, parse_filter_clause,
};

const COLUMNS: &[TableFilterColumn] = &[
    TableFilterColumn {
        code: "ord",
        label: "Order",
        kind: FilterKind::Text,
    },
    TableFilterColumn {
        code: "max",
        label: "Max",
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "coo",
        label: "Coord",
        kind: FilterKind::Coord,
    },
    TableFilterColumn {
        code: "sel",
        label: "Selected",
        kind: FilterKind::Bool,
    },
];

#[test]
fn resolves_column_codes_case_insensitively() {
    assert_eq!(
        parse_column_code(COLUMNS, "ORD").expect("column").code,
        "ord"
    );
}

#[test]
fn parses_text_contains_filter() {
    let clause = parse_filter_clause(COLUMNS[0], "bomb").expect("text filter");
    assert_eq!(clause.summary, "ORD~bomb");
    assert!(clause.predicate.matches_text(Some("Bombard World")));
    assert!(!clause.predicate.matches_text(Some("Join fleet")));
}

#[test]
fn parses_numeric_operators() {
    let clause = parse_filter_clause(COLUMNS[1], ">=50").expect("numeric filter");
    assert_eq!(
        clause.predicate,
        TableFilterPredicate::Number {
            op: NumberOp::Gte,
            value: 50
        }
    );
    assert!(clause.predicate.matches_number(Some(60)));
    assert!(!clause.predicate.matches_number(Some(40)));
}

#[test]
fn parses_unknown_numeric_match() {
    let clause = parse_filter_clause(COLUMNS[1], "?").expect("unknown filter");
    assert_eq!(clause.summary, "MAX=?");
    assert!(clause.predicate.matches_number(None));
    assert!(!clause.predicate.matches_number(Some(5)));
}

#[test]
fn parses_exact_and_radius_coordinates() {
    let exact = parse_filter_clause(COLUMNS[2], "12,7").expect("coord exact");
    assert_eq!(exact.summary, "COO=12,7");
    assert!(exact.predicate.matches_coord([12, 7]));
    assert!(!exact.predicate.matches_coord([12, 8]));

    let radius = parse_filter_clause(COLUMNS[2], "12,7/2").expect("coord radius");
    assert_eq!(radius.summary, "COO=12,7/2");
    assert!(radius.predicate.matches_coord([13, 8]));
    assert!(!radius.predicate.matches_coord([15, 7]));
}

#[test]
fn parses_boolean_aliases() {
    let yes = parse_filter_clause(COLUMNS[3], "selected").expect("bool yes");
    let no = parse_filter_clause(COLUMNS[3], "blank").expect("bool no");
    assert!(yes.predicate.matches_bool(true));
    assert!(!yes.predicate.matches_bool(false));
    assert!(no.predicate.matches_bool(false));
}

#[test]
fn char_helpers_match_expected_input_shapes() {
    assert!(is_filter_column_char('o'));
    assert!(!is_filter_column_char('1'));
    assert!(is_filter_value_char(FilterKind::Coord, '/'));
    assert!(is_filter_value_char(FilterKind::Number, '>'));
    assert!(is_filter_value_char(FilterKind::Text, '-'));
    assert!(is_filter_value_char(FilterKind::Bool, 'y'));
}
