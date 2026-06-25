//! 层叠合并单元测试。

use super::*;
use dom_flat::{ComplexSelector, CssRule};

fn rule(sel: &str, prop: &str, val: &str) -> CssRule {
    CssRule {
        selectorText: sel.to_string(),
        selector: ComplexSelector::parse(sel).unwrap(),
        style: vec![(prop.to_string(), val.to_string())],
    }
}

#[test]
fn test_cascade_single_rule() {
    let rules = vec![rule("div", "font-size", "24px")];
    let style = cascade("", &rules);
    assert_eq!(style.fontSize, 24.0);
}

#[test]
fn test_cascade_inline_overrides() {
    let rules = vec![rule("div", "font-size", "24px")];
    let style = cascade("font-size: 48px", &rules);
    assert_eq!(style.fontSize, 48.0);
}

#[test]
fn test_cascade_specificity_order() {
    let tag = rule("div", "font-size", "12px");
    let class = rule(".big", "font-size", "24px");
    let id = rule("#title", "font-size", "36px");

    let style = cascade("", &[tag.clone(), class.clone(), id.clone()]);
    assert_eq!(style.fontSize, 36.0);

    let style = cascade("", &[tag, class]);
    assert_eq!(style.fontSize, 24.0);
}

#[test]
fn test_cascade_last_wins_same_specificity() {
    let r1 = rule(".a", "color", "white");
    let r2 = rule(".b", "color", "white");

    let style1 = cascade("", &[r1.clone(), r2.clone()]);
    assert_eq!(style1.textColor, 0xFFFFFFFF);

    let style2 = cascade("", &[r1]);
    assert_eq!(style2.textColor, 0xFFFFFFFF);
}

#[test]
fn test_parse_color() {
    assert_eq!(super::parse_color("#fff"), Some(0xFFFFFFFF));
    assert_eq!(super::parse_color("#000000"), Some(0xFF000000));
    assert_eq!(super::parse_color("red"), Some(0xFFFF0000));
    assert_eq!(super::parse_color("transparent"), Some(0x00000000));
}

#[test]
fn test_parse_length() {
    assert_eq!(super::parse_length("16px"), Some(16.0));
    assert_eq!(super::parse_length("24"), Some(24.0));
    assert_eq!(super::parse_length("0"), Some(0.0));
    assert_eq!(super::parse_length(""), None);
}
