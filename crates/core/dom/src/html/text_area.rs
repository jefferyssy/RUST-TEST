//! HTMLTextAreaElement — <textarea> 元素
//!
//! 对应 W3C HTMLTextAreaElement 接口

use std::cell::RefCell;
use std::rc::Rc;

use crate::node::Node;

/// HTML <textarea> 元素
pub struct HTMLTextAreaElement {
    pub element: Rc<RefCell<Node>>,
}

impl HTMLTextAreaElement {
    pub fn new(element: Rc<RefCell<Node>>) -> Self {
        Self { element }
    }

    /// value 属性
    pub fn value(&self) -> String {
        self.element.borrow().text_content()
    }

    pub fn set_value(&self, value: &str) {
        self.element.borrow_mut().set_text_content(value);
    }

    /// placeholder 属性
    pub fn placeholder(&self) -> String {
        self.element.borrow().get_attribute("placeholder").unwrap_or_default()
    }

    pub fn set_placeholder(&self, value: &str) {
        self.element.borrow_mut().set_attribute("placeholder", value);
    }

    /// rows 属性
    pub fn rows(&self) -> u32 {
        self.element.borrow()
            .get_attribute("rows")
            .and_then(|s| s.parse().ok())
            .unwrap_or(2)
    }

    /// cols 属性
    pub fn cols(&self) -> u32 {
        self.element.borrow()
            .get_attribute("cols")
            .and_then(|s| s.parse().ok())
            .unwrap_or(20)
    }

    /// name 属性
    pub fn name(&self) -> String {
        self.element.borrow().get_attribute("name").unwrap_or_default()
    }

    pub fn set_name(&self, value: &str) {
        self.element.borrow_mut().set_attribute("name", value);
    }

    /// disabled 属性
    pub fn disabled(&self) -> bool {
        self.element.borrow().has_attribute("disabled")
    }

    pub fn set_disabled(&self, value: bool) {
        if value {
            self.element.borrow_mut().set_attribute("disabled", "");
        } else {
            self.element.borrow_mut().remove_attribute("disabled");
        }
    }

    /// readonly 属性
    pub fn read_only(&self) -> bool {
        self.element.borrow().has_attribute("readonly")
    }

    /// 选中所有文本
    pub fn select(&self) {}
}
