//! ResizeObserver — 元素尺寸变化监听 (Phase 3)
//!
//! 响应式布局核心。每帧由运行时 ObserverManager 轮询，
//! 比较元素 contentRect 变化并触发回调。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::{Node, Rect};

/// 尺寸变化条目
#[derive(Debug, Clone)]
pub struct ResizeObserverEntry {
    /// 被观察的目标元素
    pub target: Rc<RefCell<Node>>,
    /// 新尺寸
    pub content_rect: Rect<f32>,
    /// 边框盒尺寸
    pub border_box_size: (f32, f32),
    /// 内容盒尺寸
    pub content_box_size: (f32, f32),
}

/// ResizeObserver —— 元素尺寸变化监听
pub struct ResizeObserver {
    callback: Box<dyn Fn(&[ResizeObserverEntry])>,
    observed: RefCell<HashMap<usize, (Rc<RefCell<Node>>, Rect<f32>)>>,
}

impl ResizeObserver {
    /// 创建观察器
    pub fn new(callback: Box<dyn Fn(&[ResizeObserverEntry])>) -> Self {
        Self {
            callback,
            observed: RefCell::new(HashMap::new()),
        }
    }

    /// 开始观察元素
    pub fn observe(&self, target: &Rc<RefCell<Node>>) {
        let key = Rc::as_ptr(target) as usize;
        self.observed.borrow_mut().insert(key, (target.clone(), Rect::new(0.0, 0.0, 0.0, 0.0)));
    }

    /// 停止观察元素
    pub fn unobserve(&self, target: &Rc<RefCell<Node>>) {
        let key = Rc::as_ptr(target) as usize;
        self.observed.borrow_mut().remove(&key);
    }

    /// 停止所有观察
    pub fn disconnect(&self) {
        self.observed.borrow_mut().clear();
    }

    /// 每帧轮询，检查尺寸变化并触发回调
    pub fn poll(&self) {
        let mut entries = Vec::new();
        let mut observed = self.observed.borrow_mut();

        for (_key, (target, prev_rect)) in observed.iter_mut() {
            let new_rect = target.borrow().get_bounding_client_rect();
            if new_rect.width != prev_rect.width || new_rect.height != prev_rect.height {
                let entry = ResizeObserverEntry {
                    target: target.clone(),
                    content_rect: new_rect,
                    border_box_size: (new_rect.width, new_rect.height),
                    content_box_size: (new_rect.width, new_rect.height),
                };
                entries.push(entry);
                *prev_rect = new_rect;
            }
        }

        if !entries.is_empty() {
            (self.callback)(&entries);
        }
    }
}
