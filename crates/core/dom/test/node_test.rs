use super::*;
use crate::element::ElementData;
use crate::text::Text;

#[test]
fn test_new_element_node() {
    let node = Node::new(NodeType::Element(ElementData::new("div")));
    let n = node.borrow();
    assert_eq!(n.node_type(), super::node_type_constants::ELEMENT_NODE);
    assert_eq!(n.node_name(), "DIV");
    assert!(n.text_content().is_empty());
}

#[test]
fn test_new_text_node() {
    let node = Node::new(NodeType::Text(Text::new("hello")));
    let n = node.borrow();
    assert_eq!(n.node_type(), super::node_type_constants::TEXT_NODE);
    assert_eq!(n.node_name(), "#text");
    assert_eq!(n.text_content(), "hello");
}

#[test]
fn test_append_child() {
    let parent = Node::new(NodeType::Element(ElementData::new("div")));
    let child = Node::new(NodeType::Element(ElementData::new("span")));
    parent.borrow_mut().append_child(child.clone());

    assert_eq!(parent.borrow().child_nodes().len(), 1);
    assert!(child.borrow().parent_node().is_some());
}

#[test]
fn test_append_child_updates_parent() {
    let parent = Node::new(NodeType::Element(ElementData::new("div")));
    let child = Node::new(NodeType::Element(ElementData::new("span")));

    parent.borrow_mut().append_child(child.clone());
    let parent_node = child.borrow().parent_node().unwrap();
    assert_eq!(Rc::as_ptr(&parent_node), Rc::as_ptr(&parent));
}

#[test]
fn test_remove_child() {
    let parent = Node::new(NodeType::Element(ElementData::new("div")));
    let child = Node::new(NodeType::Element(ElementData::new("span")));
    parent.borrow_mut().append_child(child.clone());
    assert_eq!(parent.borrow().child_nodes().len(), 1);

    // 通过 child_nodes 获取指针，避免同时借用 child
    let child_clone = parent.borrow().child_nodes()[0].clone();
    parent.borrow_mut().remove_child_by_ptr(&child_clone);
    assert_eq!(parent.borrow().child_nodes().len(), 0);
    assert!(child_clone.borrow().parent_node().is_none());
}

#[test]
fn test_remove_non_child_panics() {
    let parent = Node::new(NodeType::Element(ElementData::new("div")));
    let child = Node::new(NodeType::Element(ElementData::new("span")));
    let child_ptr = Rc::as_ptr(&child) as *const Node;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        parent.borrow_mut().remove_child(unsafe { &*child_ptr });
    }));
    assert!(result.is_err());
}

#[test]
fn test_insert_before() {
    let parent = Node::new(NodeType::Element(ElementData::new("div")));
    let c1 = Node::new(NodeType::Element(ElementData::new("span")));
    let c2 = Node::new(NodeType::Element(ElementData::new("p")));
    parent.borrow_mut().append_child(c1.clone());
    parent.borrow_mut().append_child(c2.clone());

    let c3 = Node::new(NodeType::Element(ElementData::new("b")));
    // 先提取 c2 的裸指针，释放借用，避免 insert_before 内部 borrow_mut 冲突
    let c2_ptr: *const Node = {
        let c2_ref = c2.borrow();
        &*c2_ref as *const Node
    };
    {
        let mut p = parent.borrow_mut();
        p.insert_before(c3.clone(), Some(unsafe { &*c2_ptr }));
    }

    let children = parent.borrow().child_nodes();
    assert_eq!(children.len(), 3);
    assert_eq!(children[1].borrow().node_name(), "B");
}

#[test]
fn test_insert_before_none_appends() {
    let parent = Node::new(NodeType::Element(ElementData::new("div")));
    let c1 = Node::new(NodeType::Element(ElementData::new("span")));
    parent.borrow_mut().append_child(c1.clone());

    let c2 = Node::new(NodeType::Element(ElementData::new("p")));
    {
        let mut p = parent.borrow_mut();
        p.insert_before(c2.clone(), None);
    }
    assert_eq!(parent.borrow().child_nodes().len(), 2);
}

#[test]
fn test_replace_child() {
    let parent = Node::new(NodeType::Element(ElementData::new("div")));
    let old = Node::new(NodeType::Element(ElementData::new("span")));
    parent.borrow_mut().append_child(old.clone());

    let new = Node::new(NodeType::Element(ElementData::new("p")));
    // 先提取 old 的裸指针，释放借用，避免 replace_child 内部 borrow_mut 冲突
    let old_ptr: *const Node = {
        let old_ref = old.borrow();
        &*old_ref as *const Node
    };
    {
        let mut p = parent.borrow_mut();
        p.replace_child(new.clone(), unsafe { &*old_ptr });
    }

    assert_eq!(parent.borrow().child_nodes().len(), 1);
    assert_eq!(parent.borrow().child_nodes()[0].borrow().node_name(), "P");
    assert!(old.borrow().parent_node().is_none());
}

#[test]
fn test_contains_self() {
    let node = Node::new(NodeType::Element(ElementData::new("div")));
    let n = node.borrow();
    assert!(n.contains(&*n));
}

#[test]
fn test_contains_child() {
    let parent = Node::new(NodeType::Element(ElementData::new("div")));
    let child = Node::new(NodeType::Element(ElementData::new("span")));
    parent.borrow_mut().append_child(child.clone());

    assert!(parent.borrow().contains(&*child.borrow()));
    assert!(!child.borrow().contains(&*parent.borrow()));
}

#[test]
fn test_text_content_concat() {
    let parent = Node::new(NodeType::Element(ElementData::new("p")));
    let t1 = Node::new(NodeType::Text(Text::new("Hello ")));
    let t2 = Node::new(NodeType::Text(Text::new("World")));
    parent.borrow_mut().append_child(t1);
    parent.borrow_mut().append_child(t2);

    assert_eq!(parent.borrow().text_content(), "Hello World");
}

#[test]
fn test_set_text_content() {
    let node = Node::new(NodeType::Element(ElementData::new("div")));
    node.borrow_mut().set_text_content("hello");
    assert_eq!(node.borrow().text_content(), "hello");
    assert_eq!(node.borrow().child_nodes().len(), 1);
}

#[test]
fn test_set_text_content_overwrites() {
    let node = Node::new(NodeType::Element(ElementData::new("div")));
    node.borrow_mut().set_text_content("old");
    node.borrow_mut().set_text_content("new");
    assert_eq!(node.borrow().text_content(), "new");
    assert_eq!(node.borrow().child_nodes().len(), 1);
}

#[test]
fn test_sibling_links() {
    let parent = Node::new(NodeType::Element(ElementData::new("div")));
    let a = Node::new(NodeType::Element(ElementData::new("a")));
    let b = Node::new(NodeType::Element(ElementData::new("b")));
    parent.borrow_mut().append_child(a.clone());
    parent.borrow_mut().append_child(b.clone());

    assert!(a.borrow().previous_sibling().is_none());
    assert!(a.borrow().next_sibling().is_some());
    assert!(b.borrow().previous_sibling().is_some());
    assert!(b.borrow().next_sibling().is_none());
}

#[test]
fn test_child_element_count() {
    let parent = Node::new(NodeType::Element(ElementData::new("div")));
    parent.borrow_mut().append_child(Node::new(NodeType::Element(ElementData::new("a"))));
    parent.borrow_mut().append_child(Node::new(NodeType::Element(ElementData::new("b"))));
    assert_eq!(parent.borrow().child_element_count(), 2);
}

#[test]
fn test_element_convenience_methods() {
    let node = Node::new(NodeType::Element(ElementData::new("div")));
    node.borrow_mut().set_attribute("class", "foo");
    assert_eq!(node.borrow().get_attribute("class"), Some("foo".to_string()));
    assert!(node.borrow().has_attribute("class"));
    assert_eq!(node.borrow().tag_name(), Some("div"));

    node.borrow_mut().remove_attribute("class");
    assert!(!node.borrow().has_attribute("class"));
}

#[test]
fn test_set_style_convenience() {
    let node = Node::new(NodeType::Element(ElementData::new("div")));
    node.borrow_mut().set_style("color: red; font-size: 16px");
    // 读取 ElementData 内部 style 验证
    let saved;
    {
        let n = node.borrow();
        if let NodeType::Element(ref e) = n.node_type {
            saved = e.get_style_value("color").cloned();
        } else {
            saved = None;
        }
    }
    assert_eq!(saved, Some("red".to_string()));
}

#[test]
fn test_non_element_convenience_returns_none() {
    let text_node = Node::new(NodeType::Text(Text::new("hello")));
    assert_eq!(text_node.borrow().tag_name(), None);
    assert_eq!(text_node.borrow().get_attribute("class"), None);
}

#[test]
fn test_first_last_child() {
    let parent = Node::new(NodeType::Element(ElementData::new("div")));
    let a = Node::new(NodeType::Element(ElementData::new("a")));
    let b = Node::new(NodeType::Element(ElementData::new("b")));
    parent.borrow_mut().append_child(a.clone());
    parent.borrow_mut().append_child(b.clone());

    assert!(parent.borrow().first_child().is_some());
    assert!(parent.borrow().last_child().is_some());
    assert_eq!(
        Rc::as_ptr(&parent.borrow().first_child().unwrap()),
        Rc::as_ptr(&a)
    );
    assert_eq!(
        Rc::as_ptr(&parent.borrow().last_child().unwrap()),
        Rc::as_ptr(&b)
    );
}

#[test]
fn test_display() {
    let node = Node::new(NodeType::Element(ElementData::new("div")));
    assert_eq!(format!("{}", node.borrow()), "<div />");

    let text = Node::new(NodeType::Text(Text::new("hello")));
    assert_eq!(format!("{}", text.borrow()), "hello");
}

#[test]
fn test_document_node() {
    let node = Node::new(NodeType::Document);
    assert_eq!(node.borrow().node_type(), super::node_type_constants::DOCUMENT_NODE);
    assert_eq!(node.borrow().node_name(), "#document");
}
