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

/// 颜色类型（RGBA + 预计算线性值）
///
/// P0-7: 构造时预计算 sRGB → linear 转换，避免渲染时重复计算。
/// 使用标准 sRGB 转换公式：
///   linear = ((srgb/255 + 0.055) / 1.055)^2.4  (srgb/255 > 0.04045)
///   linear = (srgb/255) / 12.92                   (srgb/255 <= 0.04045)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
    /// 预计算的线性 R 分量 (0.0 ~ 1.0)，渲染时直接使用
    pub linear_r: f32,
    /// 预计算的线性 G 分量 (0.0 ~ 1.0)
    pub linear_g: f32,
    /// 预计算的线性 B 分量 (0.0 ~ 1.0)
    pub linear_b: f32,
}

/// sRGB 单通道 → linear 转换（标准公式）
fn srgb_to_linear(c: u8) -> f32 {
    let srgb = c as f32 / 255.0;
    if srgb > 0.04045 {
        ((srgb + 0.055) / 1.055).powf(2.4)
    } else {
        srgb / 12.92
    }
}

/// linear → sRGB 单通道转换（用于 Color::from_linear 反向计算）
fn linear_to_srgb(c: f32) -> u8 {
    let srgb = if c > 0.0031308 {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    } else {
        12.92 * c
    };
    (srgb * 255.0).round().clamp(0.0, 255.0) as u8
}

impl Color {
    /// 从 RGBA 分量创建颜色（预计算 linear 值）
    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r, g, b, a,
            linear_r: srgb_to_linear(r),
            linear_g: srgb_to_linear(g),
            linear_b: srgb_to_linear(b),
        }
    }

    /// 从 RGB 分量创建颜色（alpha=255）
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 255)
    }

    /// 从预计算的线性值创建颜色（用于 wgpu 回读）
    pub fn from_linear(linear_r: f32, linear_g: f32, linear_b: f32, a: u8) -> Self {
        Self {
            r: linear_to_srgb(linear_r),
            g: linear_to_srgb(linear_g),
            b: linear_to_srgb(linear_b),
            a,
            linear_r,
            linear_g,
            linear_b,
        }
    }

    /// 黑色 (linear: 0, 0, 0)
    pub const BLACK: Color = Color {
        r: 0, g: 0, b: 0, a: 255,
        linear_r: 0.0, linear_g: 0.0, linear_b: 0.0,
    };
    /// 白色 (linear: 1, 1, 1)
    pub const WHITE: Color = Color {
        r: 255, g: 255, b: 255, a: 255,
        linear_r: 1.0, linear_g: 1.0, linear_b: 1.0,
    };
    /// 透明 (linear: 0, 0, 0)
    pub const TRANSPARENT: Color = Color {
        r: 0, g: 0, b: 0, a: 0,
        linear_r: 0.0, linear_g: 0.0, linear_b: 0.0,
    };
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
