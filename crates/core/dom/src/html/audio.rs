//! HTMLAudioElement —— <audio> 元素 (Phase 3)

use std::cell::RefCell;
use std::rc::Rc;

use crate::Node;

/// HTMLAudioElement —— <audio src="...">
pub struct HTMLAudioElement {
    pub element: Rc<RefCell<Node>>,
}

impl HTMLAudioElement {
    pub fn new(element: Rc<RefCell<Node>>) -> Self {
        Self { element }
    }

    pub fn from_node(node: &Rc<RefCell<Node>>) -> Option<Self> {
        if node.borrow().tag_name() == Some("audio") {
            Some(Self::new(node.clone()))
        } else {
            None
        }
    }

    pub fn src(&self) -> String { self.element.borrow().get_attribute("src").unwrap_or_default() }
    pub fn set_src(&mut self, v: &str) { self.element.borrow_mut().set_attribute("src", v); }
    pub fn autoplay(&self) -> bool { self.element.borrow().has_attribute("autoplay") }
    pub fn loop_(&self) -> bool { self.element.borrow().has_attribute("loop") }
    pub fn muted(&self) -> bool { self.element.borrow().has_attribute("muted") }
    pub fn controls(&self) -> bool { self.element.borrow().has_attribute("controls") }

    pub fn current_time(&self) -> f64 { 0.0 }
    pub fn duration(&self) -> f64 { 0.0 }
    pub fn volume(&self) -> f64 { 1.0 }
    pub fn set_volume(&mut self, _v: f64) {}
    pub fn paused(&self) -> bool { true }

    pub fn play(&mut self) {}
    pub fn pause(&mut self) {}
}
