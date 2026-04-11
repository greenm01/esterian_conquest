use nc_ui::table_filter::{
    ColumnCodeParseError, FilterKind, NumberOp, TableFilterColumn, TableFilterPredicate,
    is_filter_column_char, is_filter_value_char, parse_column_code, parse_filter_clause,
};

const COLUMNS: &[TableFilterColumn] = &[
    TableFilterColumn {
        code: "ord",
        label: "Order",
        aliases: &[],
        kind: FilterKind::Text,
    },
    TableFilterColumn {
        code: "max",
        label: "Max",
        aliases: &[],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "coo",
        label: "Coord",
        aliases: &[],
        kind: FilterKind::Coord,
    },
    TableFilterColumn {
        code: "sel",
        label: "Selected",
        aliases: &[],
        kind: FilterKind::Bool,
    },
];

const PREFIX_COLUMNS: &[TableFilterColumn] = &[
    TableFilterColumn {
        code: "ord",
        label: "Order",
        aliases: &[],
        kind: FilterKind::Text,
    },
    TableFilterColumn {
        code: "max",
        label: "Max",
        aliases: &[],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "coo",
        label: "Coord",
        aliases: &[],
        kind: FilterKind::Coord,
    },
    TableFilterColumn {
        code: "sel",
        label: "Selected",
        aliases: &[],
        kind: FilterKind::Bool,
    },
    TableFilterColumn {
        code: "sco",
        label: "Scout",
        aliases: &["year", "scoutyear"],
        kind: FilterKind::Number,
    },
];

#[test]
fn resolves_column_codes_case_insensitively() {
    assert_eq!(
        parse_column_code(COLUMNS, "ORD").expect("column").code,
        "ord"
    );
    assert_eq!(parse_column_code(COLUMNS, "or").expect("prefix").code, "ord");
    assert_eq!(
        parse_column_code(COLUMNS, "selected").expect("label").code,
        "sel"
    );
}

#[test]
fn resolves_aliases_and_label_prefixes() {
    assert_eq!(parse_column_code(PREFIX_COLUMNS, "scout").expect("label").code, "sco");
    assert_eq!(parse_column_code(PREFIX_COLUMNS, "yea").expect("alias prefix").code, "sco");
}

#[test]
fn resolves_multiword_names_with_spaces_normalized() {
    const COLUMNS: &[TableFilterColumn] = &[
        TableFilterColumn {
            code: "trs",
            label: "Treasury",
            aliases: &["treasury points"],
            kind: FilterKind::Number,
        },
        TableFilterColumn {
            code: "bdg",
            label: "Budget",
            aliases: &["bdgt", "bgdt"],
            kind: FilterKind::Number,
        },
    ];

    assert_eq!(
        parse_column_code(COLUMNS, "treasury points")
            .expect("multiword alias")
            .code,
        "trs"
    );
    assert_eq!(parse_column_code(COLUMNS, "bgdt").expect("abbr alias").code, "bdg");
}

#[test]
fn reports_ambiguous_and_unknown_column_codes() {
    assert_eq!(
        parse_column_code(PREFIX_COLUMNS, "s").expect_err("ambiguous"),
        ColumnCodeParseError::Ambiguous(vec!["sco", "sel"])
    );
    assert_eq!(
        parse_column_code(PREFIX_COLUMNS, "zzz").expect_err("unknown"),
        ColumnCodeParseError::Unknown
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
    assert!(is_filter_column_char(' '));
    assert!(!is_filter_column_char('1'));
    assert!(is_filter_value_char(FilterKind::Coord, '/'));
    assert!(is_filter_value_char(FilterKind::Number, '>'));
    assert!(is_filter_value_char(FilterKind::Text, '-'));
    assert!(is_filter_value_char(FilterKind::Bool, 'y'));
}
