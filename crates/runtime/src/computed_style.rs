//! ComputedStyle — 节点最终计算样式（全小驼峰）。
//!
//! 对标浏览器 `getComputedStyle()` 返回值，仅包含一期所需的核心属性。

/// 最终计算样式。所有字段使用小驼峰，与 JS `getComputedStyle()` 一致。
///
/// 颜色字段使用 ARGB packed u32（`0xAARRGGBB`）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ComputedStyle {
    /// 背景色，ARGB packed（例如 `0xFFF5F5F5`）
    pub bgColor: u32,
    /// 文本色，ARGB packed（例如 `0xFF000000`）
    pub textColor: u32,
    /// 宽度，None 表示 auto
    pub width: Option<f32>,
    /// 高度，None 表示 auto
    pub height: Option<f32>,
    /// 内边距（px）
    pub padding: f32,
    /// 外边距（px）
    pub margin: f32,
    /// 字号（px）
    pub fontSize: f32,
    /// 字体族
    pub fontFamily: Option<String>,
    /// 文本对齐
    pub textAlign: Option<String>,
    /// 文本装饰
    pub textDecoration: Option<String>,
}

impl ComputedStyle {
    /// 从 ARGB 各分量构造 bgColor。
    pub fn bg_color_rgb(r: u8, g: u8, b: u8) -> u32 {
        (0xFFu32 << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }

    /// 从 ARGB 各分量构造 textColor。
    pub fn text_color_rgb(r: u8, g: u8, b: u8) -> u32 {
        Self::bg_color_rgb(r, g, b)
    }

    /// 提取颜色的 R/G/B 分量用于显示。
    pub fn color_to_hex(color: u32) -> String {
        format!("#{:06X}", color & 0x00FF_FFFF)
    }
}
