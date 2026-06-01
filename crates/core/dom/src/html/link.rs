//! HTMLLinkElement —— <link> 元素 (Phase 3)

use std::cell::RefCell;
use std::rc::Rc;

use crate::Node;

/// HTMLLinkElement —— <link rel="stylesheet" href="...">
pub struct HTMLLinkElement {
    pub element: Rc<RefCell<Node>>,
}

impl HTMLLinkElement {
    pub fn new(element: Rc<RefCell<Node>>) -> Self {
        Self { element }
    }

    pub fn from_node(node: &Rc<RefCell<Node>>) -> Option<Self> {
        if node.borrow().tag_name() == Some("link") {
            Some(Self::new(node.clone()))
        } else {
            None
        }
    }

    pub fn rel(&self) -> String { self.element.borrow().get_attribute("rel").unwrap_or_default() }
    pub fn href(&self) -> String { self.element.borrow().get_attribute("href").unwrap_or_default() }
    pub fn media(&self) -> String { self.element.borrow().get_attribute("media").unwrap_or_default() }
    pub fn disabled(&self) -> bool { self.element.borrow().has_attribute("disabled") }
    pub fn set_rel(&mut self, v: &str) { self.element.borrow_mut().set_attribute("rel", v); }
    pub fn set_href(&mut self, v: &str) { self.element.borrow_mut().set_attribute("href", v); }
    pub fn set_disabled(&mut self, v: bool) {
        if v {
            self.element.borrow_mut().set_attribute("disabled", "");
        } else {
            self.element.borrow_mut().remove_attribute("disabled");
        }
    }
}
