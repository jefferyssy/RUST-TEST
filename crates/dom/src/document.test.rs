use super::*;
use crate::node;

#[test]
fn test_document_new() {
    let doc = Document::new();
    let d = doc.borrow();
    // html element
    let html = d.document_element();
    assert_eq!(html.borrow().node_name(), "HTML");
    // body element
    let body = d.body();
    assert_eq!(body.borrow().node_name(), "BODY");
    // body is child of html
    assert!(html.borrow().contains(&*body.borrow()));
}

#[test]
fn test_create_element() {
    let doc = Document::new();
    let d = doc.borrow();
    let div = d.create_element("div");
    assert_eq!(div.borrow().node_name(), "DIV");
    assert_eq!(div.borrow().node_type(), node::node_type_constants::ELEMENT_NODE);
}

#[test]
fn test_create_text_node() {
    let doc = Document::new();
    let d = doc.borrow();
    let text = d.create_text_node("hello");
    assert_eq!(text.borrow().node_name(), "#text");
    assert_eq!(text.borrow().text_content(), "hello");
}
