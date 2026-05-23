//! Observer API — W3C 观察器接口 (Phase 3)
//!
//! ResizeObserver: 元素尺寸变化监听
//! IntersectionObserver: 元素可见性监听

pub mod resize_observer;
pub mod intersection_observer;

pub use resize_observer::ResizeObserver;
pub use resize_observer::ResizeObserverEntry;
pub use intersection_observer::IntersectionObserver;
pub use intersection_observer::IntersectionObserverEntry;
pub use intersection_observer::IntersectionObserverOptions;
