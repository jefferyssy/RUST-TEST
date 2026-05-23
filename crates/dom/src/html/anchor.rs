//! HTMLAnchorElement — <a> 元素
//!
//! 对应 W3C HTMLAnchorElement 接口

use std::cell::RefCell;
use std::rc::Rc;

use crate::node::Node;

/// HTML <a> 元素
pub struct HTMLAnchorElement {
    /// 底层元素节点
    pub element: Rc<RefCell<Node>>,
}

impl HTMLAnchorElement {
    /// 从 Node 包装（调用者保证是 <a> 元素）
    pub fn new(element: Rc<RefCell<Node>>) -> Self {
        Self { element }
    }

    /// href 属性
    pub fn href(&self) -> String {
        self.element.borrow().get_attribute("href").unwrap_or_default()
    }

    pub fn set_href(&self, value: &str) {
        self.element.borrow_mut().set_attribute("href", value);
    }

    /// target 属性
    pub fn target(&self) -> String {
        self.element.borrow().get_attribute("target").unwrap_or_default()
    }

    pub fn set_target(&self, value: &str) {
        self.element.borrow_mut().set_attribute("target", value);
    }

    /// rel 属性
    pub fn rel(&self) -> String {
        self.element.borrow().get_attribute("rel").unwrap_or_default()
    }

    pub fn set_rel(&self, value: &str) {
        self.element.borrow_mut().set_attribute("rel", value);
    }
}
