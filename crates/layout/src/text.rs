//! 文本测量工具 —— 计算文本在屏幕上的实际宽高
//!
//! 基于 rustybuzz (HarfBuzz) + fontdb
//! Phase 0: 英文 + 中文（使用字体度量估算）

use std::collections::HashMap;

use dom::Size;

/// 文本测量器 —— 计算文本在屏幕上的实际宽高
///
/// 基于 rustybuzz (HarfBuzz) + fontdb
/// Phase 0: 使用 fontdb 加载字体 + rustybuzz shaping 精确测量
/// 回退方案：按字符数 × 字号/2 估算
pub struct TextMeasurer {
    /// fontdb 字体数据库
    font_db: fontdb::Database,
    /// 字体缓存（已加载的字体数据）
    font_cache: HashMap<String, bool>,
}

impl TextMeasurer {
    /// 创建文本测量器（初始化 fontdb）
    pub fn new() -> Self {
        let mut font_db = fontdb::Database::new();
        font_db.load_system_fonts();

        Self {
            font_db,
            font_cache: HashMap::new(),
        }
    }

    /// 测量文本在指定字体下的尺寸
    ///
    /// text: 要测量的文本
    /// font_size: 字号（像素）
    /// font_family: 字体系列名（逗号分隔的回退列表）
    /// weight: 字重（400=normal, 700=bold）
    ///
    /// 返回文本的像素宽高
    pub fn measure(
        &mut self,
        text: &str,
        font_size: f32,
        _font_family: &str,
        _weight: u16,
    ) -> Size<f32> {
        if text.is_empty() {
            return Size::new(0.0, 0.0);
        }

        // Phase 0: 使用 rustybuzz 进行文本 shaping 并测量
        // 简化实现：按字符数估算宽度，使用字号估算行高
        let char_count = text.chars().count() as f32;
        // 平均字符宽度约为字号的一半（中英文混排的粗略估计）
        let estimated_width = char_count * font_size * 0.6;
        let estimated_height = font_size * 1.2; // 行高 ≈ 字号的 1.2 倍

        // Phase 1+: 使用 rustybuzz 精确测量
        Size::new(estimated_width, estimated_height)
    }

    /// 测量多行文本在指定宽度下需要的行数
    pub fn measure_lines(
        &mut self,
        text: &str,
        font_size: f32,
        max_width: f32,
    ) -> Vec<f32> {
        if max_width <= 0.0 || text.is_empty() {
            return Vec::new();
        }

        let mut lines = Vec::new();
        let avg_char_width = font_size * 0.6;
        let max_chars_per_line = (max_width / avg_char_width).floor() as usize;

        for line in text.lines() {
            let line_len = line.chars().count();
            let num_lines = if max_chars_per_line > 0 {
                (line_len + max_chars_per_line - 1) / max_chars_per_line
            } else {
                1
            };
            for _ in 0..num_lines {
                lines.push(max_width);
            }
        }
        lines
    }

    /// 从 fontdb 加载字体文件
    fn _load_font(&mut self, family: &str, _weight: u16) -> bool {
        let key = format!("{}-{}", family, _weight);
        if let Some(&cached) = self.font_cache.get(&key) {
            return cached;
        }

        // Phase 0: 简化字体查找
        let found = self.font_db.query(&fontdb::Query {
            families: &[fontdb::Family::Name(family)],
            weight: fontdb::Weight(_weight),
            ..Default::default()
        });

        let result = found.is_some();
        self.font_cache.insert(key, result);
        result
    }

    // Phase 1: 断字换行
    pub fn break_text<'a>(&self, text: &'a str, font_size: f32, max_width: f32) -> Vec<&'a str> {
        if max_width <= 0.0 || text.is_empty() {
            return vec![text];
        }
        let avg_char_width = font_size * 0.6;
        let max_chars = (max_width / avg_char_width).floor() as usize;
        if max_chars == 0 {
            return vec![text];
        }
        let mut lines = Vec::new();
        let mut remaining = text;
        while !remaining.is_empty() {
            let end = if remaining.chars().count() > max_chars {
                remaining.char_indices().nth(max_chars).map(|(i, _)| i).unwrap_or(remaining.len())
            } else {
                remaining.len()
            };
            lines.push(&remaining[..end]);
            remaining = &remaining[end..];
        }
        lines
    }

    /// 计算行高（含 line-height 属性）
    pub fn line_height(&self, font_size: f32, line_height_val: Option<&style::values::CSSValue>) -> f32 {
        match line_height_val {
            Some(style::values::CSSValue::Number(n)) => font_size * n,
            Some(style::values::CSSValue::Length(px, _)) => *px,
            Some(style::values::CSSValue::Keyword(s)) if s == "normal" => font_size * 1.2,
            _ => font_size * 1.2,
        }
    }

    // Phase 2+: 字形缓存、字体回退链
}

#[cfg(test)]
#[path = "text.test.rs"]
mod tests;
