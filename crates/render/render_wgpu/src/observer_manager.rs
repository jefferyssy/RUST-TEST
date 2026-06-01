//! ObserverManager —— 观察者轮询管理器 (Phase 3)
//!
//! 每个渲染帧轮询已注册的 ResizeObserver 和 IntersectionObserver，
//! 检测元素尺寸变化或可见性变化，并触发回调。
//!
//! 使用方式：
//! 1. register_resize_observer(observer)
//! 2. register_intersection_observer(observer)
//! 3. 每帧调用 poll_all(viewport)

use std::rc::Rc;

use dom::observer::resize_observer::ResizeObserver;
use dom::observer::intersection_observer::IntersectionObserver;
use dom::Rect;

/// 观察者管理器 —— 每帧轮询已注册的观察者
pub struct ObserverManager {
    resize_observers: Vec<Rc<ResizeObserver>>,
    intersection_observers: Vec<Rc<IntersectionObserver>>,
}

impl ObserverManager {
    pub fn new() -> Self {
        Self {
            resize_observers: Vec::new(),
            intersection_observers: Vec::new(),
        }
    }

    /// 注册 ResizeObserver
    pub fn register_resize_observer(&mut self, observer: Rc<ResizeObserver>) {
        self.resize_observers.push(observer);
    }

    /// 注册 IntersectionObserver
    pub fn register_intersection_observer(&mut self, observer: Rc<IntersectionObserver>) {
        self.intersection_observers.push(observer);
    }

    /// 每帧轮询所有观察者
    ///
    /// viewport: 当前视口矩形（用于 IntersectionObserver 计算）
    pub fn poll_all(&self, viewport: Rect<f32>) {
        // 轮询 ResizeObserver
        for observer in &self.resize_observers {
            observer.poll();
        }

        // 轮询 IntersectionObserver
        for observer in &self.intersection_observers {
            observer.poll(viewport);
        }
    }

    /// 获取已注册的 ResizeObserver 数量
    pub fn resize_observer_count(&self) -> usize {
        self.resize_observers.len()
    }

    /// 获取已注册的 IntersectionObserver 数量
    pub fn intersection_observer_count(&self) -> usize {
        self.intersection_observers.len()
    }

    /// 清空所有观察者
    pub fn clear(&mut self) {
        self.resize_observers.clear();
        self.intersection_observers.clear();
    }
}

impl Default for ObserverManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dom::Node;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_observer_manager_new() {
        let mgr = ObserverManager::new();
        assert_eq!(mgr.resize_observer_count(), 0);
        assert_eq!(mgr.intersection_observer_count(), 0);
    }

    #[test]
    fn test_register_resize_observer() {
        let mut mgr = ObserverManager::new();
        let node = Node::new(dom::NodeType::Element(dom::ElementData::new("div")));
        let observer = Rc::new(ResizeObserver::new(Box::new(|_| {})));
        observer.observe(&node);
        mgr.register_resize_observer(observer);
        assert_eq!(mgr.resize_observer_count(), 1);
    }

    #[test]
    fn test_register_intersection_observer() {
        let mut mgr = ObserverManager::new();
        let observer = Rc::new(IntersectionObserver::new(
            Box::new(|_| {}),
            Default::default(),
        ));
        mgr.register_intersection_observer(observer);
        assert_eq!(mgr.intersection_observer_count(), 1);
    }

    #[test]
    fn test_poll_all_does_not_panic() {
        let mut mgr = ObserverManager::new();
        let node = Node::new(dom::NodeType::Element(dom::ElementData::new("div")));

        let resize_obs = Rc::new(ResizeObserver::new(Box::new(|_| {})));
        resize_obs.observe(&node);
        mgr.register_resize_observer(resize_obs);

        let intersection_obs = Rc::new(IntersectionObserver::new(
            Box::new(|_| {}),
            Default::default(),
        ));
        intersection_obs.observe(&node);
        mgr.register_intersection_observer(intersection_obs);

        let viewport = Rect::new(0.0, 0.0, 800.0, 600.0);
        mgr.poll_all(viewport);
    }
}
