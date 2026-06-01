//! HTML 元素特化 —— Phase 2 / Phase 3
//!
//! 对应 W3C HTML Living Standard 中各 HTML 元素的专属接口。
//! 每个类型包装一个通用的 Element Node，提供类型安全的属性访问。

pub mod anchor;
pub mod audio;
pub mod image;
pub mod input;
pub mod canvas;
pub mod form;
pub mod link;
pub mod meta;
pub mod select;
pub mod text_area;
pub mod video;

pub use anchor::HTMLAnchorElement;
pub use audio::HTMLAudioElement;
pub use image::HTMLImageElement;
pub use input::HTMLInputElement;
pub use canvas::HTMLCanvasElement;
pub use form::HTMLFormElement;
pub use link::HTMLLinkElement;
pub use meta::HTMLMetaElement;
pub use select::HTMLSelectElement;
pub use text_area::HTMLTextAreaElement;
pub use video::HTMLVideoElement;
