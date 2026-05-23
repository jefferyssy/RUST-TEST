use super::*;
use crate::layout_box::{BoxType, EdgeSizes, LayoutBox};
use dom::Rect;

fn make_block(x: f32, y: f32, w: f32, h: f32) -> LayoutBox {
    let mut lb = LayoutBox::new(BoxType::Block, None);
    lb.rect = Rect::new(x, y, w, h);
    lb
}

#[test]
fn test_block_layout_empty() {
    let block = BlockLayout;
    let mut container = make_block(0.0, 0.0, 800.0, 600.0);
    block.layout(&mut container, Size::new(800.0, 600.0));
    // 无子节点，无变化
}

#[test]
fn test_block_layout_children_vertical() {
    let block = BlockLayout;
    let mut container = make_block(10.0, 10.0, 400.0, 600.0);
    container.padding = EdgeSizes::new(5.0, 5.0, 5.0, 5.0);
    container.border = EdgeSizes::new(2.0, 2.0, 2.0, 2.0);

    let mut child1 = make_block(0.0, 0.0, 100.0, 50.0);
    let mut child2 = make_block(0.0, 0.0, 100.0, 30.0);
    container.append_child(child1);
    container.append_child(child2);

    block.layout(&mut container, Size::new(800.0, 600.0));

    // child1: x = container.x + padding.left + border.left = 10 + 5 + 2 = 17
    assert_eq!(container.children[0].rect.x, 17.0);
    // child1: y = container.y + padding.top + border.top = 10 + 5 + 2 = 17
    assert_eq!(container.children[0].rect.y, 17.0);
    // child1: width = content width = 400 - 5 - 5 - 2 - 2 = 386
    assert_eq!(container.children[0].rect.width, 386.0);

    // child2: y = child1.y + child1.height = 17 + 50 = 67
    assert_eq!(container.children[1].rect.y, 67.0);
    assert_eq!(container.children[1].rect.width, 386.0);
}

#[test]
fn test_block_layout_no_padding_border() {
    let block = BlockLayout;
    let mut container = make_block(0.0, 0.0, 200.0, 200.0);

    let mut child = make_block(0.0, 0.0, 50.0, 25.0);
    container.append_child(child);

    block.layout(&mut container, Size::new(200.0, 200.0));

    assert_eq!(container.children[0].rect.x, 0.0);
    assert_eq!(container.children[0].rect.y, 0.0);
    assert_eq!(container.children[0].rect.width, 200.0);
}
