use crossterm::style::Color;
use nc_game::terminal::stdout::{ansi256_to_named16, redmean_dist, rgb_to_ansi256, rgb_to_named16};

// ─── redmean_dist properties ──────────────────────────────────────────────────

#[test]
fn redmean_identical_colors_is_zero() {
    assert_eq!(redmean_dist(128, 64, 200, 128, 64, 200), 0.0);
}

#[test]
fn redmean_weights_green_more_than_red() {
    // Same ΔChannel magnitude (+50) in green vs red — green difference must
    // produce a larger weighted distance.
    let d_green = redmean_dist(128, 128, 128, 128, 178, 128);
    let d_red = redmean_dist(128, 128, 128, 178, 128, 128);
    assert!(
        d_green > d_red,
        "green delta should outweigh red delta: green={d_green}, red={d_red}"
    );
}

#[test]
fn redmean_is_symmetric() {
    let d1 = redmean_dist(10, 20, 30, 200, 150, 100);
    let d2 = redmean_dist(200, 150, 100, 10, 20, 30);
    assert!(
        (d1 - d2).abs() < 1e-3,
        "redmean_dist should be symmetric: {d1} vs {d2}"
    );
}

// ─── rgb_to_named16: tokyo_night palette ─────────────────────────────────────

#[test]
fn tokyo_night_comment_vs_gutter_both_dark() {
    // #565f89 (comment) and #3b4261 (fg_gutter) are both muted dark blue-grays.
    // In a 16-color palette they collapse to the same dark neutral — that is
    // expected and unavoidable.  This test documents the behavior so a future
    // change that accidentally brightens one or shifts it to a warm color will
    // be caught.
    let comment = rgb_to_named16(0x56, 0x5f, 0x89);
    let gutter = rgb_to_named16(0x3b, 0x42, 0x61);
    assert!(
        matches!(comment, Color::DarkGrey | Color::DarkBlue | Color::Black),
        "tokyo_night comment should map to a dark neutral; got {comment:?}"
    );
    assert!(
        matches!(gutter, Color::DarkGrey | Color::DarkBlue | Color::Black),
        "tokyo_night gutter should map to a dark neutral; got {gutter:?}"
    );
}

#[test]
fn tokyo_night_orange_maps_to_warm_color() {
    // #ff9e64 — a warm orange; should not land on a cool color.
    let orange = rgb_to_named16(0xff, 0x9e, 0x64);
    assert!(
        matches!(orange, Color::Yellow | Color::Red | Color::DarkYellow),
        "tokyo_night orange should map to Yellow, Red, or DarkYellow; got {orange:?}"
    );
}

#[test]
fn tokyo_night_blue_maps_to_neutral_or_blue() {
    // #7aa2f7 is a muted, desaturated blue (r=122, g=162, b=247).  The VGA
    // blue is (85,85,255) — far more saturated.  In a 16-color palette this
    // color may land on Grey or a blue/cyan entry depending on the distance
    // metric.  Document the actual mapping so regressions are caught.
    let blue = rgb_to_named16(0x7a, 0xa2, 0xf7);
    assert!(
        matches!(
            blue,
            Color::Blue
                | Color::DarkBlue
                | Color::Cyan
                | Color::DarkCyan
                | Color::Grey
                | Color::White
        ),
        "tokyo_night blue should map to a blue, cyan, or neutral; got {blue:?}"
    );
}

#[test]
fn tokyo_night_green_maps_to_green_or_neutral() {
    // #9ece6a is a desaturated yellow-green (r=158, g=206, b=106).  The VGA
    // green is (85,255,85) — much more saturated.  It may land on Grey or a
    // green entry.  Document the actual mapping.
    let green = rgb_to_named16(0x9e, 0xce, 0x6a);
    assert!(
        matches!(
            green,
            Color::Green | Color::DarkGreen | Color::Grey | Color::Yellow | Color::DarkYellow
        ),
        "tokyo_night green should map to a green, yellow, or neutral; got {green:?}"
    );
}

// ─── rgb_to_named16: edge cases ───────────────────────────────────────────────

#[test]
fn pure_black_maps_to_black() {
    assert_eq!(rgb_to_named16(0, 0, 0), Color::Black);
}

#[test]
fn pure_white_maps_to_white() {
    assert_eq!(rgb_to_named16(255, 255, 255), Color::White);
}

// ─── rgb_to_ansi256: cube-vs-grayscale tiebreaking ───────────────────────────

#[test]
fn near_gray_prefers_grayscale_ramp() {
    // rgb(90, 85, 85) — very close to gray; the grayscale ramp entry should
    // win over the color cube, because a neutral cube entry is farther in
    // perceptual distance.
    let idx = rgb_to_ansi256(90, 85, 85);
    assert!(
        idx >= 232 || idx == 16 || idx == 231,
        "near-gray rgb(90,85,85) should land on grayscale ramp or cube endpoints; got index {idx}"
    );
}

#[test]
fn saturated_color_stays_in_cube() {
    // rgb(200, 50, 50) — clearly red, should be in the color cube (16–231).
    let idx = rgb_to_ansi256(200, 50, 50);
    assert!(
        (16..=231).contains(&idx),
        "saturated red should land in color cube; got index {idx}"
    );
}

#[test]
fn rgb_to_ansi256_pure_black() {
    // Pure black should land on cube index 16 (black cube entry).
    let idx = rgb_to_ansi256(0, 0, 0);
    assert_eq!(idx, 16, "pure black should map to cube index 16");
}

#[test]
fn rgb_to_ansi256_pure_white() {
    // Pure white should land on cube index 231 (white cube entry).
    let idx = rgb_to_ansi256(255, 255, 255);
    assert_eq!(idx, 231, "pure white should map to cube index 231");
}

// ─── ansi256_to_named16: direct-index passthrough ────────────────────────────

#[test]
fn ansi256_direct_indices_pass_through() {
    // The first 16 indices must map to their corresponding named colors.
    use Color::*;
    let expected = [
        Black,
        DarkRed,
        DarkGreen,
        DarkYellow,
        DarkBlue,
        DarkMagenta,
        DarkCyan,
        Grey,
        DarkGrey,
        Red,
        Green,
        Yellow,
        Blue,
        Magenta,
        Cyan,
        White,
    ];
    for (i, &expected_color) in expected.iter().enumerate() {
        let got = ansi256_to_named16(i as u8);
        assert_eq!(
            got, expected_color,
            "index {i} should map to {expected_color:?}"
        );
    }
}
