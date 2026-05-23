use super::*;
use dom::Color;

#[test]
fn test_parse_keyword() {
    let v = parse_css_value("display", "none");
    assert_eq!(v, CSSValue::Keyword("none".to_string()));
}

#[test]
fn test_parse_hex_color_3digit() {
    let v = parse_css_value("color", "#f00");
    assert_eq!(v, CSSValue::Color(Color::rgb(255, 0, 0)));
}

#[test]
fn test_parse_hex_color_6digit() {
    let v = parse_css_value("color", "#ff8800");
    assert_eq!(v, CSSValue::Color(Color::rgb(255, 136, 0)));
}

#[test]
fn test_parse_rgb_color() {
    let v = parse_css_value("color", "rgb(255, 0, 128)");
    assert_eq!(v, CSSValue::Color(Color::rgb(255, 0, 128)));
}

#[test]
fn test_parse_rgba_color() {
    let v = parse_css_value("color", "rgba(255, 0, 0, 0.5)");
    assert_eq!(v, CSSValue::Color(Color::rgba(255, 0, 0, 127)));
}

#[test]
fn test_parse_named_color() {
    assert_eq!(parse_css_value("color", "red"), CSSValue::Color(Color::rgb(255, 0, 0)));
    assert_eq!(parse_css_value("color", "blue"), CSSValue::Color(Color::rgb(0, 0, 255)));
    assert_eq!(parse_css_value("color", "transparent"), CSSValue::Color(Color::TRANSPARENT));
}

#[test]
fn test_parse_length_px() {
    let v = parse_css_value("width", "100px");
    assert_eq!(v, CSSValue::Length(100.0, CSSUnit::Px));
}

#[test]
fn test_parse_length_percent() {
    let v = parse_css_value("width", "50%");
    assert_eq!(v, CSSValue::Length(50.0, CSSUnit::Percent));
}

#[test]
fn test_parse_length_em() {
    let v = parse_css_value("font-size", "1.5em");
    assert_eq!(v, CSSValue::Length(1.5, CSSUnit::Em));
}

#[test]
fn test_parse_length_rem() {
    let v = parse_css_value("font-size", "2rem");
    assert_eq!(v, CSSValue::Length(2.0, CSSUnit::Rem));
}

#[test]
fn test_parse_zero_length() {
    let v = parse_css_value("margin", "0");
    assert_eq!(v, CSSValue::Length(0.0, CSSUnit::Px));
}

#[test]
fn test_parse_color_function() {
    assert_eq!(parse_color("#ff0"), Color::rgb(255, 255, 0));
    assert_eq!(parse_color("red"), Color::rgb(255, 0, 0));
    assert_eq!(parse_color("unknown"), Color::BLACK);
}

#[test]
fn test_parse_length_function() {
    assert_eq!(parse_length("42px"), (42.0, CSSUnit::Px));
    assert_eq!(parse_length("invalid"), (0.0, CSSUnit::Px));
}

#[test]
fn test_named_colors() {
    assert_eq!(named_color("black"), Some(Color::rgb(0, 0, 0)));
    assert_eq!(named_color("white"), Some(Color::rgb(255, 255, 255)));
    assert_eq!(named_color("green"), Some(Color::rgb(0, 128, 0)));
    assert_eq!(named_color("orange"), Some(Color::rgb(255, 165, 0)));
    assert_eq!(named_color("purple"), Some(Color::rgb(128, 0, 128)));
    assert_eq!(named_color("gray"), Some(Color::rgb(128, 128, 128)));
    assert_eq!(named_color("grey"), Some(Color::rgb(128, 128, 128)));
    assert_eq!(named_color("unknown_color"), None);
}
