//! # renderer crate — 最终渲染 + 跨平台运行时
//!
//! 整合 GPU 渲染后端、窗口管理、事件循环、Observer 管理器。
//! 提供完整的渲染管线和应用入口。
//!
//! Phase 3: wgpu 后端 + Window trait + ObserverManager + HitTester

pub mod wgpu_backend;
pub mod text_renderer;
pub mod window;
pub mod window_trait;
pub mod event_loop;
pub mod hit_test;
pub mod observer_manager;

pub use wgpu_backend::{WgpuBackend, TextureAtlas};
pub use text_renderer::TextRenderer;
pub use window::WebWindow;
pub use window_trait::{Window, ViewportInfo};
pub use event_loop::AnimationFrameScheduler;
pub use hit_test::HitTester;
pub use observer_manager::ObserverManager;

use render_tree::DisplayList;

/// sRGB → 线性色彩空间转换
///
/// CSS 颜色定义在 sRGB 空间，但 GPU 管线通常输出到 sRGB 格式的 surface，
/// 硬件会自动将线性值转换为 sRGB 编码。因此需要在 shader 输入前做反向转换。
pub(crate) fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// 渲染后端接口 —— 可插拔
///
/// 所有渲染后端都实现这个 trait
pub trait RenderBackend {
    /// 渲染一帧
    fn render(&mut self, display_list: &DisplayList);
    /// 窗口尺寸变更通知
    fn resize(&mut self, width: u32, height: u32);
    /// 交换缓冲区（显示帧）
    fn present(&mut self);
    /// 获取当前渲染尺寸
    fn size(&self) -> (u32, u32);
}

#[cfg(test)]
#[path = "lib.test.rs"]
mod tests;
