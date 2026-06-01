use super::*;
use crate::layout_box::LayoutBox;
use style::values::CSSValue;
use dom::Rect;

#[test]
fn test_parse_position_value_length() {
    let val = CSSValue::Length(10.0, style::values::CSSUnit::Px);
    assert_eq!(PositionedLayout::parse_position_value(Some(&val)), 10.0);
}

#[test]
fn test_parse_position_value_auto() {
    let val = CSSValue::Keyword("auto".to_string());
    assert_eq!(PositionedLayout::parse_position_value(Some(&val)), 0.0);
}

#[test]
fn test_parse_position_value_none() {
    assert_eq!(PositionedLayout::parse_position_value(None), 0.0);
}

#[test]
fn test_relative_offset_zero() {
    let positioned = PositionedLayout;
    let mut root = LayoutBox::new(crate::layout_box::BoxType::Block, None);
    root.rect = Rect::new(0.0, 0.0, 100.0, 100.0);
    // 没有 computed_style，不会发生偏移
    positioned.layout(&mut root, Size::new(800.0, 600.0));
    assert_eq!(root.rect.x, 0.0);
    assert_eq!(root.rect.y, 0.0);
}
