use super::*;
use crate::command::PaintCommand;
use style::cascade::ComputedStyle;
use style::values::{CSSUnit, CSSValue};
use dom::{Color, Rect};
use layout::layout_box::{BoxType, LayoutBox};

fn build_text_layout(text: &str, x: f32, y: f32, font_size: f32) -> LayoutBox {
    let node = dom::Node::new(dom::NodeType::Text(dom::text::Text::new(text)));
    let mut lb = LayoutBox::new(BoxType::Text, Some(node));
    lb.rect = Rect::new(x, y, 100.0, 20.0);
    let mut style = ComputedStyle::new();
    style.set("font-size", CSSValue::Length(font_size, CSSUnit::Px));
    style.set("color", CSSValue::Color(Color::rgb(0, 0, 0)));
    lb.computed_style = Some(style);
    lb
}

fn build_block_with_style(
    x: f32, y: f32, w: f32, h: f32,
    bg: Option<Color>,
) -> LayoutBox {
    let mut lb = LayoutBox::new(BoxType::Block, None);
    lb.rect = Rect::new(x, y, w, h);
    if let Some(color) = bg {
        let mut style = ComputedStyle::new();
        style.set("background-color", CSSValue::Color(color));
        lb.computed_style = Some(style);
    }
    lb
}

#[test]
fn test_build_empty() {
    let mut builder = DisplayListBuilder::new();
    let root = LayoutBox::new(BoxType::Block, None);
    let dl = builder.build(&root);
    assert!(dl.is_empty());
}

#[test]
fn test_build_background() {
    let mut builder = DisplayListBuilder::new();
    let root = build_block_with_style(10.0, 20.0, 200.0, 100.0, Some(Color::rgb(255, 0, 0)));
    let dl = builder.build(&root);
    assert_eq!(dl.len(), 1);
    match &dl.commands()[0] {
        PaintCommand::FillRect { rect, color, .. } => {
            assert_eq!(rect.x, 10.0);
            assert_eq!(rect.y, 20.0);
            assert_eq!(rect.width, 200.0);
            assert_eq!(rect.height, 100.0);
            assert_eq!(*color, Color::rgb(255, 0, 0));
        }
        other => panic!("expected FillRect, got {other:?}"),
    }
}

#[test]
fn test_build_no_background() {
    let mut builder = DisplayListBuilder::new();
    let root = build_block_with_style(0.0, 0.0, 100.0, 100.0, None);
    let dl = builder.build(&root);
    // 没有背景色，也没有文本 → 无命令
    assert!(dl.is_empty());
}

#[test]
fn test_build_text() {
    let mut builder = DisplayListBuilder::new();
    let root = build_text_layout("Hello", 10.0, 20.0, 16.0);
    let dl = builder.build(&root);
    assert!(!dl.is_empty());
    match &dl.commands()[0] {
        PaintCommand::Text { text, font_size, x, y, .. } => {
            assert_eq!(text, "Hello");
            assert_eq!(*font_size, 16.0);
            assert_eq!(*x, 10.0);
            // builder intern: y = text_rect.y + half_leading = 20 + (20-16)/2 = 22
            assert_eq!(*y, 22.0);
        }
        other => panic!("expected Text, got {other:?}"),
    }
}

#[test]
fn test_build_nested() {
    let mut builder = DisplayListBuilder::new();
    let mut child = build_block_with_style(0.0, 0.0, 50.0, 50.0, Some(Color::rgb(0, 0, 255)));
    child.children.push(build_text_layout("nested", 5.0, 5.0, 12.0));

    let mut root = build_block_with_style(0.0, 0.0, 200.0, 200.0, Some(Color::rgb(255, 0, 0)));
    root.children.push(child);

    let dl = builder.build(&root);
    // 3 commands: root bg + child bg + text
    assert_eq!(dl.len(), 3);
}
