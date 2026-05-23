//! HTMLInputElement — <input> 元素
//!
//! 对应 W3C HTMLInputElement 接口

use std::cell::RefCell;
use std::rc::Rc;

use crate::node::Node;

/// HTML <input> 元素
pub struct HTMLInputElement {
    pub element: Rc<RefCell<Node>>,
}

impl HTMLInputElement {
    pub fn new(element: Rc<RefCell<Node>>) -> Self {
        Self { element }
    }

    /// type 属性
    pub fn input_type(&self) -> String {
        self.element.borrow().get_attribute("type").unwrap_or_else(|| "text".to_string())
    }

    pub fn set_input_type(&self, value: &str) {
        self.element.borrow_mut().set_attribute("type", value);
    }

    /// value 属性
    pub fn value(&self) -> String {
        self.element.borrow().get_attribute("value").unwrap_or_default()
    }

    pub fn set_value(&self, value: &str) {
        self.element.borrow_mut().set_attribute("value", value);
    }

    /// placeholder 属性
    pub fn placeholder(&self) -> String {
        self.element.borrow().get_attribute("placeholder").unwrap_or_default()
    }

    pub fn set_placeholder(&self, value: &str) {
        self.element.borrow_mut().set_attribute("placeholder", value);
    }

    /// checked 属性
    pub fn checked(&self) -> bool {
        self.element.borrow().has_attribute("checked")
    }

    pub fn set_checked(&self, value: bool) {
        if value {
            self.element.borrow_mut().set_attribute("checked", "");
        } else {
            self.element.borrow_mut().remove_attribute("checked");
        }
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

    /// name 属性
    pub fn name(&self) -> String {
        self.element.borrow().get_attribute("name").unwrap_or_default()
    }

    pub fn set_name(&self, value: &str) {
        self.element.borrow_mut().set_attribute("name", value);
    }

    /// 选中输入内容
    /// Phase 2: stub
    pub fn select(&self) {}
}
