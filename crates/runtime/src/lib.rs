//! runtime — 应用运行时（DOM 存储 + CSS 引擎 + 应用入口）。
//!
//! 全小驼峰命名，1:1 对齐原生 JS DOM 属性（`parentNode`, `childNodes`, `classList` 等）。
//! DomRegistry 即运行时 `document` 全局对象。
//! AppRegistry 提供统一的链式 API：`new()` → `loadNodes()` → `loadStyles()` → `init()`。
//!
//! 可视化调试功能（DOM 树打印、JSON 序列化、DevTools 仪表盘）已提取到 `devtools` crate。

#![allow(non_snake_case)]

// ── DOM 存储 ──
pub mod node_id;
pub mod dom_node;
pub mod dom_registry;
pub mod computed_style;
pub mod vnode;
pub mod dom_ops;
pub mod query;

// ── CSSOM 类型 ──
pub mod specificity;
pub mod selector_match;
pub mod select;
pub mod css_rule;
pub mod style_sheet;

// ── CSS 引擎 ──
pub mod index;
pub mod cascade;
pub mod style_manager;

// ── 应用入口 ──
pub mod app_registry;
pub mod plugin;

// ── DOM 重导出 ──
pub use node_id::NodeId;
pub use dom_node::DomNode;
pub use dom_registry::DomRegistry;
pub use computed_style::ComputedStyle;
pub use vnode::{VNode, VElementVNode};
pub use dom_ops::DomNodeOps;
pub use query::DomQuery;

// ── CSSOM 重导出 ──
pub use specificity::Specificity;
pub use selector_match::{
    AttrOp, BasicSelector, Combinator, ComplexSelector,
    PseudoClass, PseudoElement, SelectSegment, SelectorMatchExt,
};
pub use select::Select;
pub use css_rule::CssRule;
pub use style_sheet::{CssStyleSheet, StyleSheetList};

// ── CSS 引擎重导出 ──
pub use index::RuleIndex;
pub use cascade::cascade;
pub use style_manager::{StyleEngine, parse_css};

// ── 应用入口重导出 ──
pub use app_registry::AppRegistry;
pub use plugin::Plugin;

/// 矩形结构（全小驼峰）
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// 占位 Event 类型（一期：仅满足 addEventListener 签名）。
///
/// 后续对接完整的 DOM Events 规范（bubbles, cancelable, target 等）。
#[derive(Debug, Clone)]
pub struct Event {
    /// 事件类型（如 "click", "input"）
    pub eventType: String,
}
