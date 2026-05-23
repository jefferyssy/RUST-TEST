//! HTMLMetaElement —— <meta> 元素 (Phase 3)

use std::cell::RefCell;
use std::rc::Rc;

use crate::Node;

/// HTMLMetaElement —— <meta charset="utf-8">
pub struct HTMLMetaElement {
    pub element: Rc<RefCell<Node>>,
}

impl HTMLMetaElement {
    pub fn new(element: Rc<RefCell<Node>>) -> Self {
        Self { element }
    }

    pub fn from_node(node: &Rc<RefCell<Node>>) -> Option<Self> {
        if node.borrow().tag_name() == Some("meta") {
            Some(Self::new(node.clone()))
        } else {
            None
        }
    }

    pub fn name(&self) -> String { self.element.borrow().get_attribute("name").unwrap_or_default() }
    pub fn content(&self) -> String { self.element.borrow().get_attribute("content").unwrap_or_default() }
    pub fn charset(&self) -> String { self.element.borrow().get_attribute("charset").unwrap_or_default() }
    pub fn set_name(&mut self, v: &str) { self.element.borrow_mut().set_attribute("name", v); }
    pub fn set_content(&mut self, v: &str) { self.element.borrow_mut().set_attribute("content", v); }
}
