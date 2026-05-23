//! 跨平台 Window trait —— Phase 3
//!
//! 统一抽象不同平台的窗口创建：
//! - Desktop: winit + wgpu
//! - WASM: web-sys HtmlCanvasElement + WebGPU
//! - iOS: Metal
//! - Android: Vulkan

use dom::Size;

/// 跨平台窗口抽象
///
/// 每个平台提供各自的实现，调用方通过 trait 方法操作窗口。
pub trait Window {
    /// 获取窗口尺寸（CSS 像素）
    fn size(&self) -> Size<f32>;

    /// 请求重绘下一帧
    fn request_redraw(&self);

    /// 获取设备像素比 (DPR)
    fn device_pixel_ratio(&self) -> f32;

    /// 设置窗口标题
    fn set_title(&mut self, title: &str);

    /// 获取视口尺寸信息
    fn viewport_info(&self) -> ViewportInfo;
}

/// 视口信息（用于媒体查询 + Observer 计算）
#[derive(Debug, Clone)]
pub struct ViewportInfo {
    pub width: f32,
    pub height: f32,
    pub device_pixel_ratio: f32,
}

impl Default for ViewportInfo {
    fn default() -> Self {
        Self {
            width: 800.0,
            height: 600.0,
            device_pixel_ratio: 1.0,
        }
    }
}
