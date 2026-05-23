//! HTMLFormElement — <form> 元素
//!
//! 对应 W3C HTMLFormElement 接口

use std::cell::RefCell;
use std::rc::Rc;

use crate::node::{Node, NodeType};

/// HTML <form> 元素
pub struct HTMLFormElement {
    pub element: Rc<RefCell<Node>>,
}

impl HTMLFormElement {
    pub fn new(element: Rc<RefCell<Node>>) -> Self {
        Self { element }
    }

    /// action 属性
    pub fn action(&self) -> String {
        self.element.borrow().get_attribute("action").unwrap_or_default()
    }

    pub fn set_action(&self, value: &str) {
        self.element.borrow_mut().set_attribute("action", value);
    }

    /// method 属性
    pub fn method(&self) -> String {
        self.element.borrow().get_attribute("method").unwrap_or_else(|| "get".to_string())
    }

    pub fn set_method(&self, value: &str) {
        self.element.borrow_mut().set_attribute("method", value);
    }

    /// enctype 属性
    pub fn enctype(&self) -> String {
        self.element.borrow()
            .get_attribute("enctype")
            .unwrap_or_else(|| "application/x-www-form-urlencoded".to_string())
    }

    /// 表单内所有可提交元素的 name=value 集合
    /// Phase 2: 收集 input/select/textarea 子元素的值
    pub fn elements(&self) -> Vec<(String, String)> {
        let mut result = Vec::new();
        self.collect_form_elements(&self.element, &mut result);
        result
    }

    fn collect_form_elements(&self, node: &Rc<RefCell<Node>>, result: &mut Vec<(String, String)>) {
        let n = node.borrow();
        if let NodeType::Element(e) = &n.node_type {
            let tag = e.tag_name();
            if tag == "input" || tag == "select" || tag == "textarea" {
                if let Some(name) = e.get_attribute("name") {
                    let value = e.get_attribute("value").unwrap_or_default();
                    result.push((name, value));
                }
            }
        }
        for child in &n.children {
            self.collect_form_elements(child, result);
        }
    }

    /// 提交表单
    /// Phase 2: stub（Phase 3+ 调用 fetch/net crate）
    pub fn submit(&self) {}

    /// 重置表单
    pub fn reset(&self) {
        // Phase 2: 基础 stub
    }
}
