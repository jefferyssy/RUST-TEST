use super::*;
use dom::{Color, Rect};

#[test]
fn test_display_list_new() {
    let dl = DisplayList::new();
    assert!(dl.is_empty());
    assert_eq!(dl.len(), 0);
}

#[test]
fn test_push_and_commands() {
    let mut dl = DisplayList::new();
    dl.push(PaintCommand::FillRect {
        rect: Rect::new(0.0, 0.0, 100.0, 50.0),
        color: Color::rgb(255, 0, 0),
    });
    assert_eq!(dl.len(), 1);
    assert!(!dl.is_empty());
    assert_eq!(dl.commands().len(), 1);
}

#[test]
fn test_clear() {
    let mut dl = DisplayList::new();
    dl.push(PaintCommand::FillRect {
        rect: Rect::new(0.0, 0.0, 10.0, 10.0),
        color: Color::WHITE,
    });
    assert_eq!(dl.len(), 1);
    dl.clear();
    assert!(dl.is_empty());
}

#[test]
fn test_multiple_commands() {
    let mut dl = DisplayList::new();
    dl.push(PaintCommand::FillRect {
        rect: Rect::new(0.0, 0.0, 100.0, 100.0),
        color: Color::rgb(255, 0, 0),
    });
    dl.push(PaintCommand::Border {
        rect: Rect::new(0.0, 0.0, 100.0, 100.0),
        widths: [1.0; 4], colors: [Color::BLACK; 4], radius: 0.0,
        style: BorderStyle::Solid,
    });
    dl.push(PaintCommand::Text {
        text: "hello".to_string(),
        font_size: 16.0,
        font_family: "sans-serif".to_string(),
        font_weight: 400,
        x: 10.0, y: 20.0,
        color: Color::BLACK,
        decoration: TextDecoration::None,
    });
    assert_eq!(dl.len(), 3);
    match &dl.commands()[0] {
        PaintCommand::FillRect { color, .. } => assert_eq!(*color, Color::rgb(255, 0, 0)),
        _ => panic!("expected FillRect"),
    }
}

#[test]
fn test_sort_by_z_order() {
    let mut dl = DisplayList::new();
    dl.push(PaintCommand::FillRect {
        rect: Rect::new(0.0, 0.0, 10.0, 10.0),
        color: Color::rgb(255, 0, 0),
    });
    // sort is a no-op in Phase 0, just ensure it doesn't panic
    dl.sort_by_z_order();
    assert_eq!(dl.len(), 1);
}
