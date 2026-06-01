//! HTMLSelectElement — <select> 元素
//!
//! 对应 W3C HTMLSelectElement 接口

use std::cell::RefCell;
use std::rc::Rc;

use crate::node::{Node, NodeType};

/// HTML <select> 元素
pub struct HTMLSelectElement {
    pub element: Rc<RefCell<Node>>,
}

impl HTMLSelectElement {
    pub fn new(element: Rc<RefCell<Node>>) -> Self {
        Self { element }
    }

    /// 当前选中值
    pub fn value(&self) -> String {
        let node = self.element.borrow();
        if let NodeType::Element(e) = &node.node_type {
            if let Some(v) = e.get_attribute("value") {
                return v;
            }
        }
        // 查找第一个 selected <option>
        for child in &node.children {
            if let Some(val) = Self::option_value(child) {
                return val;
            }
        }
        String::new()
    }

    fn option_value(opt: &Rc<RefCell<Node>>) -> Option<String> {
        let n = opt.borrow();
        if let NodeType::Element(e) = &n.node_type {
            if e.tag_name() == "option" {
                if e.has_attribute("selected") {
                    return e.get_attribute("value")
                        .or_else(|| Some(n.text_content()));
                }
            }
        }
        None
    }

    /// selectedIndex
    pub fn selected_index(&self) -> i32 {
        let node = self.element.borrow();
        for (i, child) in node.children.iter().enumerate() {
            let n = child.borrow();
            if let NodeType::Element(e) = &n.node_type {
                if e.tag_name() == "option" && e.has_attribute("selected") {
                    return i as i32;
                }
            }
        }
        -1
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

    /// multiple 属性
    pub fn multiple(&self) -> bool {
        self.element.borrow().has_attribute("multiple")
    }
}
