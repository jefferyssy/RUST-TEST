use super::*;

fn make_text_box(text: &str) -> LayoutBox {
    let node = dom::Node::new(dom::NodeType::Text(dom::text::Text::new(text)));
    LayoutBox::new(BoxType::Text, Some(node))
}

fn make_block_box() -> LayoutBox {
    LayoutBox::new(BoxType::Block, None)
}

#[test]
fn test_new_layout_box() {
    let lb = LayoutBox::new(BoxType::Block, None);
    assert_eq!(lb.box_type, BoxType::Block);
    assert!(lb.node.is_none());
    assert!(lb.children.is_empty());
    assert_eq!(lb.rect, Rect::new(0.0, 0.0, 0.0, 0.0));
}

#[test]
fn test_append_child() {
    let mut parent = make_block_box();
    let child = make_text_box("hello");
    parent.append_child(child);
    assert_eq!(parent.children.len(), 1);
}

#[test]
fn test_content_area_no_padding() {
    let lb = LayoutBox::new(BoxType::Block, None);
    let area = lb.content_area();
    assert_eq!(area.width, 0.0);
    assert_eq!(area.height, 0.0);
}

#[test]
fn test_content_area_with_padding() {
    let mut lb = make_block_box();
    lb.rect = Rect::new(0.0, 0.0, 200.0, 100.0);
    lb.padding = EdgeSizes::new(10.0, 20.0, 10.0, 20.0);
    let area = lb.content_area();
    assert_eq!(area.width, 160.0);  // 200 - 20 - 20
    assert_eq!(area.height, 80.0);  // 100 - 10 - 10
}

#[test]
fn test_content_area_with_border() {
    let mut lb = make_block_box();
    lb.rect = Rect::new(0.0, 0.0, 100.0, 50.0);
    lb.padding = EdgeSizes::new(5.0, 5.0, 5.0, 5.0);
    lb.border = EdgeSizes::new(2.0, 2.0, 2.0, 2.0);
    let area = lb.content_area();
    assert_eq!(area.width, 86.0);   // 100 - 5 - 5 - 2 - 2
    assert_eq!(area.height, 36.0);  // 50 - 5 - 5 - 2 - 2
}

#[test]
fn test_set_content_area() {
    let mut lb = make_block_box();
    lb.padding = EdgeSizes::new(10.0, 10.0, 10.0, 10.0);
    lb.border = EdgeSizes::new(1.0, 1.0, 1.0, 1.0);
    lb.set_content_area(Size::new(100.0, 50.0));
    assert_eq!(lb.rect.width, 122.0);  // 100 + 10 + 10 + 1 + 1
    assert_eq!(lb.rect.height, 72.0);  // 50 + 10 + 10 + 1 + 1
}

#[test]
fn test_border_box() {
    let mut lb = make_block_box();
    lb.rect = Rect::new(10.0, 20.0, 200.0, 100.0);
    let bb = lb.border_box();
    assert_eq!(bb.width, 200.0);
    assert_eq!(bb.height, 100.0);
}

#[test]
fn test_margin_box() {
    let mut lb = make_block_box();
    lb.rect = Rect::new(0.0, 0.0, 100.0, 50.0);
    lb.margin = EdgeSizes::new(10.0, 10.0, 10.0, 10.0);
    let mb = lb.margin_box();
    assert_eq!(mb.width, 120.0);
    assert_eq!(mb.height, 70.0);
}

#[test]
fn test_traverse() {
    let mut root = make_block_box();
    let child1 = make_text_box("a");
    let child2 = make_text_box("b");
    root.append_child(child1);
    root.append_child(child2);

    let mut count = 0;
    root.traverse(&mut |_| count += 1);
    assert_eq!(count, 3); // root + 2 children
}

#[test]
fn test_find() {
    let mut root = make_block_box();
    root.append_child(make_text_box("hello"));
    root.append_child(make_text_box("world"));

    let texts = root.find(&|lb| lb.box_type == BoxType::Text);
    assert_eq!(texts.len(), 2);
}
