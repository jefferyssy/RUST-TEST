//! flex 模块测试 — FlexLayout 辅助函数

use super::*;
use style::values::{CSSValue, CSSUnit};
use style::ComputedStyle;

// ============================================================
//  resolve_length 测试
// ============================================================

#[test]
fn test_resolve_length_from_length_value() {
    let val = CSSValue::Length(42.0, CSSUnit::Px);
    assert_eq!(resolve_length(&val), Some(42.0));
}

#[test]
fn test_resolve_length_from_px_keyword() {
    let val = CSSValue::Keyword("16px".into());
    assert_eq!(resolve_length(&val), Some(16.0));
}

#[test]
fn test_resolve_length_from_other_keyword() {
    let val = CSSValue::Keyword("auto".into());
    assert_eq!(resolve_length(&val), None);
}

#[test]
fn test_resolve_length_from_number() {
    let val = CSSValue::Number(42.0);
    assert_eq!(resolve_length(&val), None);
}

#[test]
fn test_resolve_length_zero() {
    let val = CSSValue::Length(0.0, CSSUnit::Px);
    assert_eq!(resolve_length(&val), Some(0.0));
}

// ============================================================
//  css_keyword 测试
// ============================================================

#[test]
fn test_css_keyword_returns_string() {
    assert_eq!(css_keyword(&CSSValue::Keyword("flex".into())), "flex");
    assert_eq!(css_keyword(&CSSValue::Keyword("center".into())), "center");
}

#[test]
fn test_css_keyword_non_keyword() {
    assert_eq!(css_keyword(&CSSValue::Number(1.0)), "");
    assert_eq!(css_keyword(&CSSValue::Length(10.0, CSSUnit::Px)), "");
}

// ============================================================
//  keyword_eq 测试
// ============================================================

#[test]
fn test_keyword_eq_match() {
    assert!(keyword_eq(&CSSValue::Keyword("auto".into()), "auto"));
}

#[test]
fn test_keyword_eq_mismatch() {
    assert!(!keyword_eq(&CSSValue::Keyword("flex".into()), "auto"));
}

#[test]
fn test_keyword_eq_non_keyword() {
    assert!(!keyword_eq(&CSSValue::Number(1.0), "auto"));
}

// ============================================================
//  css_number 测试
// ============================================================

#[test]
fn test_css_number_from_number() {
    assert_eq!(css_number(&CSSValue::Number(3.0)), Some(3.0));
    assert_eq!(css_number(&CSSValue::Number(0.0)), Some(0.0));
    assert_eq!(css_number(&CSSValue::Number(-1.0)), Some(-1.0));
}

#[test]
fn test_css_number_from_parsable_keyword() {
    assert_eq!(css_number(&CSSValue::Keyword("5".into())), Some(5.0));
    assert_eq!(css_number(&CSSValue::Keyword("2.5".into())), Some(2.5));
}

#[test]
fn test_css_number_from_unparsable_keyword() {
    assert_eq!(css_number(&CSSValue::Keyword("abc".into())), None);
    assert_eq!(css_number(&CSSValue::Keyword("auto".into())), None);
}

#[test]
fn test_css_number_from_other() {
    assert_eq!(css_number(&CSSValue::Length(10.0, CSSUnit::Px)), None);
}

// ============================================================
//  is_border_box_style 测试
// ============================================================

#[test]
fn test_is_border_box_true() {
    let mut cs = ComputedStyle::new();
    cs.set("box-sizing", CSSValue::Keyword("border-box".into()));
    assert!(is_border_box_style(&cs));
}

#[test]
fn test_is_border_box_false_when_not_set() {
    let cs = ComputedStyle::new();
    assert!(!is_border_box_style(&cs));
}

#[test]
fn test_is_border_box_false_for_content_box() {
    let mut cs = ComputedStyle::new();
    cs.set("box-sizing", CSSValue::Keyword("content-box".into()));
    assert!(!is_border_box_style(&cs));
}
