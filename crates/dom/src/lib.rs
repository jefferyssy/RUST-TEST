//! # DOM crate — W3C DOM 标准实现
//!
//! 符合 W3C DOM Living Standard 规范的 Rust 实现。
//! Phase 0 实现核心节点类型 + 树操作 + 基础事件。
//! Phase 1 新增 DocumentFragment, Comment, 完整事件系统, MutationObserver。
//!
//! 使用方式：
//! ```rust
//! use dom::*;
//! let doc = Document::new();
//! let div = doc.borrow().create_element("div");
//! ```

pub mod node;
pub mod element;
pub mod document;
pub mod text;
pub mod event;
pub mod dom_token_list;
pub mod mutation_observer;
pub mod html;
pub mod observer;

// Phase 0 公开类型
pub use node::Node;
pub use node::NodeType;
pub use node::node_type_constants;
pub use element::ElementData;
pub use document::Document;
pub use text::Text;
pub use event::{
    Event,
    EventPhase,
    EventListenerOptions,
    MouseEvent,
    KeyboardEvent,
    FocusEvent,
    WheelEvent,
    WheelDeltaMode,
    AnimationEvent,
    TransitionEvent,
    InputEvent,
    EventDispatcher,
    Touch,
    TouchList,
    TouchEvent,
    PointerEvent,
    PointerType,
    CustomEvent,
};
pub use dom_token_list::DOMTokenList;
pub use mutation_observer::{
    MutationObserver,
    MutationRecord,
    MutationRecordType,
    MutationObserverInit,
};
pub use html::{
    HTMLAnchorElement,
    HTMLAudioElement,
    HTMLImageElement,
    HTMLInputElement,
    HTMLCanvasElement,
    HTMLFormElement,
    HTMLLinkElement,
    HTMLMetaElement,
    HTMLSelectElement,
    HTMLTextAreaElement,
    HTMLVideoElement,
};

/// W3C DocumentPosition 位掩码常量
pub mod document_position {
    /// 两个节点在不同文档中
    pub const DOCUMENT_POSITION_DISCONNECTED: u16 = 1;
    /// other 在 this 之前（文档顺序）
    pub const DOCUMENT_POSITION_PRECEDING: u16 = 2;
    /// other 在 this 之后（文档顺序）
    pub const DOCUMENT_POSITION_FOLLOWING: u16 = 4;
    /// other 是 this 的后代
    pub const DOCUMENT_POSITION_CONTAINS: u16 = 8;
    /// other 是 this 的祖先
    pub const DOCUMENT_POSITION_CONTAINED_BY: u16 = 16;
}

/// 颜色类型（RGBA）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    /// 从 RGBA 分量创建颜色
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// 从 RGB 分量创建颜色（alpha=255）
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// 黑色
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    /// 白色
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    /// 透明
    pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);
}

/// 矩形类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect<T> {
    pub x: T,
    pub y: T,
    pub width: T,
    pub height: T,
}

impl<T> Rect<T> {
    pub const fn new(x: T, y: T, width: T, height: T) -> Self {
        Self { x, y, width, height }
    }
}

/// 尺寸类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size<T> {
    pub width: T,
    pub height: T,
}

impl<T> Size<T> {
    pub const fn new(width: T, height: T) -> Self {
        Self { width, height }
    }
}

/// 二维坐标 (Phase 1 新增)
#[derive(Debug, Clone, Copy)]
pub struct Point<T> {
    pub x: T,
    pub y: T,
}

impl<T> Point<T> {
    pub const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

/// 四边尺寸 (Phase 1 新增)
#[derive(Debug, Clone, Copy)]
pub struct EdgeInsets<T> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

impl<T> EdgeInsets<T> {
    pub const fn new(top: T, right: T, bottom: T, left: T) -> Self {
        Self { top, right, bottom, left }
    }
}

#[cfg(test)]
#[path = "lib.test.rs"]
mod tests;
