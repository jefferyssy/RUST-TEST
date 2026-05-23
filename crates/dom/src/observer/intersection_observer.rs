//! IntersectionObserver — 元素可见性监听 (Phase 3)
//!
//! 懒加载、无限滚动、曝光埋点的基石。
//! 每帧由运行时 ObserverManager 轮询，计算与视口的交叉比。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::{Node, Rect};

/// IntersectionObserver 配置选项
#[derive(Debug, Clone)]
pub struct IntersectionObserverOptions {
    pub threshold: f32,
    pub root_margin: (f32, f32, f32, f32),
}

impl Default for IntersectionObserverOptions {
    fn default() -> Self {
        Self {
            threshold: 0.0,
            root_margin: (0.0, 0.0, 0.0, 0.0),
        }
    }
}

/// 可见性变化条目
#[derive(Debug, Clone)]
pub struct IntersectionObserverEntry {
    pub target: Rc<RefCell<Node>>,
    pub intersection_ratio: f32,
    pub is_intersecting: bool,
    pub intersection_rect: Rect<f32>,
    pub bounding_client_rect: Rect<f32>,
    pub root_bounds: Rect<f32>,
    pub time: f64,
}

/// IntersectionObserver —— 元素可见性监听
pub struct IntersectionObserver {
    callback: Box<dyn Fn(&[IntersectionObserverEntry])>,
    options: IntersectionObserverOptions,
    observed: RefCell<HashMap<usize, Rc<RefCell<Node>>>>,
}

impl IntersectionObserver {
    pub fn new(
        callback: Box<dyn Fn(&[IntersectionObserverEntry])>,
        options: IntersectionObserverOptions,
    ) -> Self {
        Self {
            callback,
            options,
            observed: RefCell::new(HashMap::new()),
        }
    }

    pub fn observe(&self, target: &Rc<RefCell<Node>>) {
        let key = Rc::as_ptr(target) as usize;
        self.observed.borrow_mut().insert(key, target.clone());
    }

    pub fn unobserve(&self, target: &Rc<RefCell<Node>>) {
        let key = Rc::as_ptr(target) as usize;
        self.observed.borrow_mut().remove(&key);
    }

    pub fn disconnect(&self) {
        self.observed.borrow_mut().clear();
    }

    /// 每帧轮询，计算交叉比并触发回调
    pub fn poll(&self, viewport: Rect<f32>) {
        let mut entries = Vec::new();
        let observed = self.observed.borrow();
        let now = web_time_now();

        for (_key, target) in observed.iter() {
            let target_rect = target.borrow().get_bounding_client_rect();

            // 计算与视口的交集
            let intersect_x = target_rect.x.max(viewport.x);
            let intersect_y = target_rect.y.max(viewport.y);
            let intersect_w = (target_rect.x + target_rect.width).min(viewport.x + viewport.width) - intersect_x;
            let intersect_h = (target_rect.y + target_rect.height).min(viewport.y + viewport.height) - intersect_y;

            let intersect_area = if intersect_w > 0.0 && intersect_h > 0.0 {
                intersect_w * intersect_h
            } else {
                0.0
            };

            let target_area = target_rect.width * target_rect.height;
            let ratio = if target_area > 0.0 {
                intersect_area / target_area
            } else {
                0.0
            };

            let is_intersecting = ratio > self.options.threshold;

            entries.push(IntersectionObserverEntry {
                target: target.clone(),
                intersection_ratio: ratio,
                is_intersecting,
                intersection_rect: Rect::new(intersect_x, intersect_y, intersect_w.max(0.0), intersect_h.max(0.0)),
                bounding_client_rect: target_rect,
                root_bounds: viewport,
                time: now,
            });
        }

        if !entries.is_empty() {
            (self.callback)(&entries);
        }
    }
}

fn web_time_now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as f64
}
