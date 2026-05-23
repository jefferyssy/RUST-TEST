//! HTMLImageElement — <img> 元素
//!
//! 对应 W3C HTMLImageElement 接口

use std::cell::RefCell;
use std::rc::Rc;

use crate::node::Node;

/// HTML <img> 元素
pub struct HTMLImageElement {
    pub element: Rc<RefCell<Node>>,
}

impl HTMLImageElement {
    pub fn new(element: Rc<RefCell<Node>>) -> Self {
        Self { element }
    }

    /// src 属性
    pub fn src(&self) -> String {
        self.element.borrow().get_attribute("src").unwrap_or_default()
    }

    pub fn set_src(&self, value: &str) {
        self.element.borrow_mut().set_attribute("src", value);
    }

    /// alt 属性
    pub fn alt(&self) -> String {
        self.element.borrow().get_attribute("alt").unwrap_or_default()
    }

    pub fn set_alt(&self, value: &str) {
        self.element.borrow_mut().set_attribute("alt", value);
    }

    /// width 属性（自然宽度）
    pub fn width(&self) -> u32 {
        self.element.borrow()
            .get_attribute("width")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    }

    /// height 属性（自然高度）
    pub fn height(&self) -> u32 {
        self.element.borrow()
            .get_attribute("height")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    }

    /// complete 属性（图片是否加载完成）
    /// Phase 2: 基础 stub，始终返回 true
    pub fn complete(&self) -> bool {
        true
    }

    /// naturalWidth —— 图片自然宽度
    /// Phase 2: 返回 width 属性值
    pub fn natural_width(&self) -> u32 {
        self.width()
    }

    /// naturalHeight —— 图片自然高度
    /// Phase 2: 返回 height 属性值
    pub fn natural_height(&self) -> u32 {
        self.height()
    }
}
