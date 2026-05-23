//! HTMLCanvasElement — <canvas> 元素
//!
//! 对应 W3C HTMLCanvasElement 接口

use std::cell::RefCell;
use std::rc::Rc;

use crate::node::Node;

/// HTML <canvas> 元素
pub struct HTMLCanvasElement {
    pub element: Rc<RefCell<Node>>,
}

impl HTMLCanvasElement {
    pub fn new(element: Rc<RefCell<Node>>) -> Self {
        Self { element }
    }

    /// width 属性
    pub fn width(&self) -> u32 {
        self.element.borrow()
            .get_attribute("width")
            .and_then(|s| s.parse().ok())
            .unwrap_or(300)
    }

    pub fn set_width(&self, value: u32) {
        self.element.borrow_mut().set_attribute("width", &value.to_string());
    }

    /// height 属性
    pub fn height(&self) -> u32 {
        self.element.borrow()
            .get_attribute("height")
            .and_then(|s| s.parse().ok())
            .unwrap_or(150)
    }

    pub fn set_height(&self, value: u32) {
        self.element.borrow_mut().set_attribute("height", &value.to_string());
    }

    /// 获取 2D 渲染上下文
    /// Phase 2: stub，返回 None（Phase 3+ 完整实现 CanvasRenderingContext2D）
    pub fn get_context(&self, _context_type: &str) -> Option<()> {
        // Phase 3+: CanvasRenderingContext2D
        None
    }

    /// 导出为 data URL
    /// Phase 2: stub
    pub fn to_data_url(&self, _format: &str, _quality: f32) -> String {
        String::new()
    }
}
