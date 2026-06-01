//! computed 模块测试 — ComputedStyle CoW 语义

use super::*;
use crate::values::CSSUnit;

#[test]
fn test_new_is_empty() {
    let style = ComputedStyle::new();
    assert!(style.is_empty());
}

#[test]
fn test_default_is_empty() {
    let style = ComputedStyle::default();
    assert!(style.is_empty());
}

#[test]
fn test_set_and_get_string() {
    let mut style = ComputedStyle::new();
    style.set("color", CSSValue::Keyword("red".into()));
    assert_eq!(style.get("color"), Some(&CSSValue::Keyword("red".into())));
}

#[test]
fn test_set_and_get_id() {
    let mut style = ComputedStyle::new();
    style.set_id(PropertyId::Width, CSSValue::Length(100.0, CSSUnit::Px));
    assert_eq!(style.get_id(PropertyId::Width), Some(&CSSValue::Length(100.0, CSSUnit::Px)));
}

#[test]
fn test_get_nonexistent() {
    let style = ComputedStyle::new();
    assert_eq!(style.get("width"), None);
    assert_eq!(style.get_id(PropertyId::Width), None);
}

#[test]
fn test_contains_key() {
    let mut style = ComputedStyle::new();
    assert!(!style.contains_key("width"));
    style.set("width", CSSValue::Length(200.0, CSSUnit::Px));
    assert!(style.contains_key("width"));
}

#[test]
fn test_contains_id() {
    let mut style = ComputedStyle::new();
    assert!(!style.contains_id(PropertyId::Width));
    style.set_id(PropertyId::Width, CSSValue::Length(200.0, CSSUnit::Px));
    assert!(style.contains_id(PropertyId::Width));
}

#[test]
fn test_get_or_initial_missing() {
    let style = ComputedStyle::new();
    assert_eq!(style.get_or_initial("nonexistent"), CSSValue::Initial);
}

#[test]
fn test_get_or_initial_present() {
    let mut style = ComputedStyle::new();
    style.set("color", CSSValue::Keyword("blue".into()));
    assert_eq!(style.get_or_initial("color"), CSSValue::Keyword("blue".into()));
}

#[test]
fn test_set_unknown_property_is_noop() {
    let mut style = ComputedStyle::new();
    style.set("not-a-real-property", CSSValue::Keyword("test".into()));
    assert!(style.is_empty());
}

#[test]
fn test_merge_adds_new_properties() {
    let mut base = ComputedStyle::new();
    base.set_id(PropertyId::Width, CSSValue::Length(100.0, CSSUnit::Px));

    let mut other = ComputedStyle::new();
    other.set_id(PropertyId::Color, CSSValue::Keyword("red".into()));

    base.merge(&other);
    assert!(base.contains_id(PropertyId::Width));
    assert!(base.contains_id(PropertyId::Color));
}

#[test]
fn test_merge_does_not_override() {
    let mut base = ComputedStyle::new();
    base.set_id(PropertyId::Color, CSSValue::Keyword("blue".into()));

    let mut other = ComputedStyle::new();
    other.set_id(PropertyId::Color, CSSValue::Keyword("red".into()));

    base.merge(&other);
    assert_eq!(base.get_id(PropertyId::Color), Some(&CSSValue::Keyword("blue".into())));
}

#[test]
fn test_clone_equality() {
    let mut style = ComputedStyle::new();
    style.set_id(PropertyId::Color, CSSValue::Keyword("green".into()));
    let cloned = style.clone();
    assert_eq!(style, cloned);
}

#[test]
fn test_clone_independence() {
    let mut original = ComputedStyle::new();
    original.set_id(PropertyId::Color, CSSValue::Keyword("red".into()));

    let mut cloned = original.clone();
    cloned.set_id(PropertyId::Color, CSSValue::Keyword("blue".into()));

    // CoW: original 不受 cloned 修改影响
    assert_eq!(original.get_id(PropertyId::Color), Some(&CSSValue::Keyword("red".into())));
    assert_eq!(cloned.get_id(PropertyId::Color), Some(&CSSValue::Keyword("blue".into())));
}

#[test]
fn test_is_empty_after_clear_merge() {
    let style = ComputedStyle::new();
    let mut merged = ComputedStyle::new();
    merged.merge(&style);
    assert!(merged.is_empty());
}

#[test]
fn test_iter_collects_all() {
    let mut style = ComputedStyle::new();
    style.set_id(PropertyId::Width, CSSValue::Length(100.0, CSSUnit::Px));
    style.set_id(PropertyId::Color, CSSValue::Keyword("red".into()));
    style.set_id(PropertyId::Display, CSSValue::Keyword("flex".into()));

    let items: Vec<_> = style.iter().collect();
    assert_eq!(items.len(), 3);
}

#[test]
fn test_multiple_groups_coalesce() {
    let mut style = ComputedStyle::new();
    // Group 0 (box model) + Group 1 (visual) + Group 2 (text)
    style.set_id(PropertyId::Width, CSSValue::Length(50.0, CSSUnit::Px));
    style.set_id(PropertyId::Color, CSSValue::Keyword("red".into()));
    style.set_id(PropertyId::FontSize, CSSValue::Length(16.0, CSSUnit::Px));

    assert!(style.contains_id(PropertyId::Width));
    assert!(style.contains_id(PropertyId::Color));
    assert!(style.contains_id(PropertyId::FontSize));
}

// ============================================================
//  StyleDiff 测试
// ============================================================

#[test]
fn test_diff_identical_styles() {
    let mut a = ComputedStyle::new();
    a.set_id(PropertyId::Width, CSSValue::Length(100.0, CSSUnit::Px));
    let b = a.clone();

    let diff = diff_styles(&a, &b);
    assert!(diff.changed.is_empty());
    assert!(!diff.layout_affecting);
    assert!(!diff.paint_affecting);
}

#[test]
fn test_diff_layout_property_changed() {
    let mut a = ComputedStyle::new();
    a.set_id(PropertyId::Width, CSSValue::Length(100.0, CSSUnit::Px));

    let mut b = a.clone();
    b.set_id(PropertyId::Width, CSSValue::Length(200.0, CSSUnit::Px));

    let diff = diff_styles(&a, &b);
    assert!(diff.changed.contains(&PropertyId::Width));
    assert!(diff.layout_affecting);
}

#[test]
fn test_diff_paint_property_changed() {
    let mut a = ComputedStyle::new();
    a.set_id(PropertyId::Color, CSSValue::Keyword("red".into()));

    let mut b = a.clone();
    b.set_id(PropertyId::Color, CSSValue::Keyword("blue".into()));

    let diff = diff_styles(&a, &b);
    assert!(diff.changed.contains(&PropertyId::Color));
    assert!(diff.paint_affecting);
}

#[test]
fn test_diff_property_added() {
    let a = ComputedStyle::new();
    let mut b = ComputedStyle::new();
    b.set_id(PropertyId::Margin, CSSValue::Length(10.0, CSSUnit::Px));

    let diff = diff_styles(&a, &b);
    assert!(diff.changed.contains(&PropertyId::Margin));
    assert!(diff.layout_affecting);
}

#[test]
fn test_diff_property_removed() {
    let mut a = ComputedStyle::new();
    a.set_id(PropertyId::FontSize, CSSValue::Length(16.0, CSSUnit::Px));
    let b = ComputedStyle::new();

    let diff = diff_styles(&a, &b);
    assert!(diff.changed.contains(&PropertyId::FontSize));
    assert!(diff.layout_affecting);
}

#[test]
fn test_diff_empty_styles() {
    let a = ComputedStyle::new();
    let b = ComputedStyle::new();
    let diff = diff_styles(&a, &b);
    assert!(diff.changed.is_empty());
}
