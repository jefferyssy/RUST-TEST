//! 文本测量工具 —— 计算文本在屏幕上的实际宽高
//!
//! 参考 Chrome Blink 引擎方式：fontdb 查询系统字体 → rustybuzz (HarfBuzz) 塑形 →
//! 累加 glyph x_advance 获取精确宽度。
//!
//! 回退方案：若字体未加载，使用字符感知估算（char_width_factor）。

use std::collections::HashMap;

use ab_glyph::Font;
use dom::Size;

/// 字体缓存条目
struct CachedFont {
    /// 字体文件原始字节
    data: Vec<u8>,
    /// 字体索引（ttc 文件）
    face_index: u32,
}

/// 文本测量器 —— 计算文本在屏幕上的实际宽高
///
/// 基于 rustybuzz (HarfBuzz) + fontdb，使用系统字体精确测量。
pub struct TextMeasurer {
    /// fontdb 字体数据库
    font_db: fontdb::Database,
    /// 已加载的字体缓存（family-weight → 字体数据）
    font_cache: HashMap<String, CachedFont>,
}

impl TextMeasurer {
    /// 创建文本测量器（初始化 fontdb 并加载系统字体）
    pub fn new() -> Self {
        let mut font_db = fontdb::Database::new();
        font_db.load_system_fonts();

        Self {
            font_db,
            font_cache: HashMap::new(),
        }
    }

    /// 测量文本在指定字体下的精确像素宽度
    ///
    /// 使用 rustybuzz 塑形，累加 glyph x_advance 获取总宽度。
    /// 若字体不可用，回退到字符感知估算。
    pub fn measure_width(
        &mut self,
        text: &str,
        font_size: f32,
        font_family: &str,
        font_weight: u16,
    ) -> f32 {
        if text.is_empty() {
            return 0.0;
        }

        // 尝试加载字体并使用 rustybuzz 塑形
        if let Some(width) = self.measure_with_font(text, font_size, font_family, font_weight) {
            return width;
        }

        // 回退：字符感知估算
        text.chars().map(|c| char_width_factor(c) * font_size).sum()
    }

    /// 使用 rustybuzz 塑形测量文本宽度
    fn measure_with_font(
        &mut self,
        text: &str,
        font_size: f32,
        font_family: &str,
        font_weight: u16,
    ) -> Option<f32> {
        let (data, face_index) = self.load_font(font_family, font_weight)?;

        let face = rustybuzz::Face::from_slice(data, face_index)?;
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(text);
        let shaped = rustybuzz::shape(&face, &[], buffer);

        // 与 text_renderer 保持一致：使用 ab_glyph 的 height_unscaled 作为缩放基准
        let ag_font = ab_glyph::FontRef::try_from_slice(data).ok()?;
        let height = ag_font.height_unscaled();
        if height <= 0.0 {
            return None;
        }
        let scale = font_size / height;

        // 累加所有字形的 x_advance
        let total_advance: i32 = shaped.glyph_positions().iter().map(|p| p.x_advance).sum();
        Some(total_advance as f32 * scale)
    }

    /// 加载字体数据（按 family + weight），缓存结果
    /// 支持逗号分隔的字体族名（CSS font-family fallback 机制）
    fn load_font(&mut self, family: &str, weight: u16) -> Option<(&[u8], u32)> {
        let key = format!("{}-{}", family, weight);

        // 检查缓存
        if !self.font_cache.contains_key(&key) {
            // 拆分逗号分隔的字体族名，逐个尝试
            let families: Vec<&str> = family
                .split(',')
                .map(|s| s.trim().trim_matches('"').trim_matches('\''))
                .filter(|s| !s.is_empty())
                .collect();

            for fam in &families {
                let fontdb_family = match *fam {
                    "serif" => fontdb::Family::Serif,
                    "sans-serif" => fontdb::Family::SansSerif,
                    "monospace" => fontdb::Family::Monospace,
                    "cursive" => fontdb::Family::Cursive,
                    "fantasy" => fontdb::Family::Fantasy,
                    _ => fontdb::Family::Name(fam),
                };

                let query = self.font_db.query(&fontdb::Query {
                    families: &[fontdb_family],
                    weight: fontdb::Weight(weight),
                    ..Default::default()
                });

                if let Some(face_id) = query {
                    if let Some(face_info) = self.font_db.face(face_id) {
                        let path = match &face_info.source {
                            fontdb::Source::File(p) | fontdb::Source::SharedFile(p, _) => {
                                Some(p.clone())
                            }
                            _ => None,
                        };

                        if let Some(ref p) = path {
                            if let Ok(data) = std::fs::read(p) {
                                self.font_cache.insert(
                                    key.clone(),
                                    CachedFont {
                                        data,
                                        face_index: face_info.index,
                                    },
                                );
                                break; // 找到第一个可用字体即停止
                            }
                        }
                    }
                }
            }
        }

        // 从缓存返回引用
        self.font_cache.get(&key).map(|f| (f.data.as_slice(), f.face_index))
    }

    /// 测量文本在指定字体下的尺寸（宽+高）
    pub fn measure(
        &mut self,
        text: &str,
        font_size: f32,
        font_family: &str,
        weight: u16,
    ) -> Size<f32> {
        if text.is_empty() {
            return Size::new(0.0, 0.0);
        }
        let width = self.measure_width(text, font_size, font_family, weight);
        let height = font_size * 1.2; // 行高 ≈ 字号的 1.2 倍
        Size::new(width, height)
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
        let avg_char_width = font_size * 0.54;
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

    /// Phase 1: 断字换行
    pub fn break_text<'a>(&self, text: &'a str, font_size: f32, max_width: f32) -> Vec<&'a str> {
        if max_width <= 0.0 || text.is_empty() {
            return vec![text];
        }
        let avg_char_width = font_size * 0.54;
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
}

/// Phase 0: 字符感知的宽度系数（sans-serif 近似，作为字体不可用时的回退方案）
///
/// 不同字符在比例字体中的宽度差异很大，使用单一系数会导致
/// 短文本（如 "v1.0"）和含空格句子（如 "Click a nav item"）估计偏差过大。
pub fn char_width_factor(c: char) -> f32 {
    match c {
        // 大写宽字符
        'M' | 'W' => 0.80,
        // 大写正常
        'A'..='Z' => 0.68,
        // 小写宽字符
        'm' | 'w' => 0.72,
        // 小写窄字符
        'i' | 'j' | 'l' | 'f' | 't' | 'r' => 0.38,
        // 空格
        ' ' => 0.28,
        // 标点
        '.' | ',' | ':' | ';' | '!' | '?' | '\'' | '"' | '/' | '\\' | '-' | '(' | ')' => 0.32,
        // 数字和其余小写
        _ => 0.54,
    }
}

#[cfg(test)]
#[path = "text.test.rs"]
mod tests;
