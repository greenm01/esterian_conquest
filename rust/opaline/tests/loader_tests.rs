use opaline::error::OpalineError;
use opaline::loader;
use opaline::{OpalineColor, OpalineStyle};
use pretty_assertions::assert_eq;

const MINIMAL_KDL: &str = r##"
meta name="Minimal" variant="dark"
palette "red" "#ff0000"
palette "blue" "#0000ff"
token "accent.primary" "red"
token "accent.secondary" "blue"
style "keyword" fg="accent.primary" bold=#true
gradient "primary" "red" "blue"
"##;

#[test]
fn load_minimal_theme_from_string() {
    let theme = loader::load_from_str(MINIMAL_KDL, None).expect("valid KDL");
    assert_eq!(theme.meta.name, "Minimal");
    assert!(theme.is_dark());
}

#[test]
fn loaded_theme_resolves_tokens() {
    let theme = loader::load_from_str(MINIMAL_KDL, None).expect("valid KDL");
    assert_eq!(theme.color("accent.primary"), OpalineColor::new(255, 0, 0));
    assert_eq!(
        theme.color("accent.secondary"),
        OpalineColor::new(0, 0, 255)
    );
}

#[test]
fn loaded_theme_resolves_styles() {
    let theme = loader::load_from_str(MINIMAL_KDL, None).expect("valid KDL");
    let style = theme.style("keyword");
    assert_eq!(style, OpalineStyle::fg(OpalineColor::new(255, 0, 0)).bold());
}

#[test]
fn loaded_theme_resolves_gradients() {
    let theme = loader::load_from_str(MINIMAL_KDL, None).expect("valid KDL");
    assert_eq!(theme.gradient("primary", 0.0), OpalineColor::new(255, 0, 0));
    assert_eq!(theme.gradient("primary", 1.0), OpalineColor::new(0, 0, 255));
}

#[test]
fn missing_token_returns_fallback() {
    let theme = loader::load_from_str(MINIMAL_KDL, None).expect("valid KDL");
    assert_eq!(theme.color("nonexistent"), OpalineColor::FALLBACK);
}

#[test]
fn missing_style_returns_default() {
    let theme = loader::load_from_str(MINIMAL_KDL, None).expect("valid KDL");
    assert_eq!(theme.style("nonexistent"), OpalineStyle::default());
}

#[test]
fn missing_gradient_returns_fallback() {
    let theme = loader::load_from_str(MINIMAL_KDL, None).expect("valid KDL");
    assert_eq!(theme.gradient("nonexistent", 0.5), OpalineColor::FALLBACK);
}

#[test]
fn has_token_checks() {
    let theme = loader::load_from_str(MINIMAL_KDL, None).expect("valid KDL");
    assert!(theme.has_token("accent.primary"));
    assert!(!theme.has_token("nonexistent"));
}

#[test]
fn has_style_checks() {
    let theme = loader::load_from_str(MINIMAL_KDL, None).expect("valid KDL");
    assert!(theme.has_style("keyword"));
    assert!(!theme.has_style("nonexistent"));
}

#[test]
fn has_gradient_checks() {
    let theme = loader::load_from_str(MINIMAL_KDL, None).expect("valid KDL");
    assert!(theme.has_gradient("primary"));
    assert!(!theme.has_gradient("nonexistent"));
}

#[test]
fn invalid_kdl_returns_parse_error() {
    let err = loader::load_from_str("this is not kdl {{{", None).expect_err("should fail");
    assert!(matches!(err, OpalineError::Parse { .. }));
}

#[test]
fn load_from_file_nonexistent_returns_io_error() {
    let err = loader::load_from_file(std::path::Path::new("/tmp/opaline_nonexistent.kdl"))
        .expect_err("should fail");
    assert!(matches!(err, OpalineError::Io { .. }));
}

#[test]
fn theme_token_names() {
    let theme = loader::load_from_str(MINIMAL_KDL, None).expect("valid KDL");
    let names = theme.token_names();
    assert!(names.contains(&"accent.primary"));
    assert!(names.contains(&"accent.secondary"));
}

#[test]
fn theme_style_names() {
    let theme = loader::load_from_str(MINIMAL_KDL, None).expect("valid KDL");
    let names = theme.style_names();
    assert!(names.contains(&"keyword"));
}

#[test]
fn theme_gradient_names() {
    let theme = loader::load_from_str(MINIMAL_KDL, None).expect("valid KDL");
    let names = theme.gradient_names();
    assert!(names.contains(&"primary"));
}

#[test]
fn light_theme_variant() {
    let kdl = r#"
meta name="Light Test" variant="light"
"#;
    let theme = loader::load_from_str(kdl, None).expect("valid KDL");
    assert!(theme.is_light());
    assert!(!theme.is_dark());
}

#[test]
fn empty_gradient_array_returns_error() {
    let kdl = r#"
meta name="Empty Gradient"
gradient "primary"
"#;

    let err = loader::load_from_str(kdl, None).expect_err("should fail");
    assert!(matches!(err, OpalineError::EmptyGradient));
}

#[test]
fn unknown_top_level_field_is_rejected() {
    let kdl = r#"
oops "nope"
meta name="Unknown Top Level"
"#;

    let err = loader::load_from_str(kdl, None).expect_err("should fail");
    assert!(matches!(err, OpalineError::Parse { .. }));
}

#[test]
fn unknown_meta_field_is_rejected() {
    let kdl = r#"
meta name="Unknown Meta" variant="dark" unexpected="nope"
"#;

    let err = loader::load_from_str(kdl, None).expect_err("should fail");
    assert!(matches!(err, OpalineError::Parse { .. }));
}

#[test]
fn unknown_style_field_is_rejected() {
    let kdl = r#"
meta name="Unknown Style"
style "keyword" fg="accent.primary" bold=#true unexpected=#true
"#;

    let err = loader::load_from_str(kdl, None).expect_err("should fail");
    assert!(matches!(err, OpalineError::Parse { .. }));
}
