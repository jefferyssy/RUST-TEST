//! CSS 属性解析 — Phase 3 新增属性
//!
//! 为 Phase 3 新增的 CSS 属性提供解析函数。
//! 每个函数将 CSS 字符串值转换为 crate::values::CSSValue。

use crate::values::CSSValue;

// ============================================================
//  布局属性
// ============================================================

/// aspect-ratio: 宽高比约束
/// 值: auto | <ratio>
/// 例: aspect-ratio: 16/9
pub fn parse_aspect_ratio(value: &str) -> CSSValue {
    let value = value.trim();
    if value == "auto" {
        return CSSValue::Keyword("auto".to_string());
    }
    // 解析 "16/9" 格式
    if let Some((w_str, h_str)) = value.split_once('/') {
        let w: f32 = w_str.trim().parse().unwrap_or(1.0);
        let h: f32 = h_str.trim().parse().unwrap_or(1.0);
        if h != 0.0 {
            return CSSValue::Composite(vec![
                CSSValue::Number(w),
                CSSValue::Number(h),
            ]);
        }
    }
    // 尝试解析为单个数值
    if let Ok(n) = value.parse::<f32>() {
        return CSSValue::Number(n);
    }
    CSSValue::Keyword(value.to_string())
}

/// contain: 渲染隔离
/// 值: none | strict | content | [size || layout || style || paint]
pub fn parse_contain(value: &str) -> CSSValue {
    CSSValue::Keyword(value.trim().to_string())
}

/// content-visibility: 可见性渲染控制
/// 值: visible | auto | hidden
pub fn parse_content_visibility(value: &str) -> CSSValue {
    match value.trim() {
        "visible" => CSSValue::Keyword("visible".to_string()),
        "auto" => CSSValue::Keyword("auto".to_string()),
        "hidden" => CSSValue::Keyword("hidden".to_string()),
        other => CSSValue::Keyword(other.to_string()),
    }
}

// ============================================================
//  盒模型属性
// ============================================================

/// outline-offset: outline 偏移距离
/// 值: <length>
pub fn parse_outline_offset(value: &str) -> CSSValue {
    crate::values::parse_css_value("outline-offset", value)
}

// ============================================================
//  排版属性
// ============================================================

/// font-variant: 字体变体
/// 值: normal | small-caps | ...
pub fn parse_font_variant(value: &str) -> CSSValue {
    CSSValue::Keyword(value.trim().to_string())
}

/// font-stretch: 字体拉伸
/// 值: normal | ultra-condensed | extra-condensed | condensed |
///      semi-condensed | semi-expanded | expanded | extra-expanded |
///      ultra-expanded | <percentage>
pub fn parse_font_stretch(value: &str) -> CSSValue {
    CSSValue::Keyword(value.trim().to_string())
}

/// word-break: 断词规则
/// 值: normal | break-all | keep-all | break-word
pub fn parse_word_break(value: &str) -> CSSValue {
    match value.trim() {
        "normal" => CSSValue::Keyword("normal".to_string()),
        "break-all" => CSSValue::Keyword("break-all".to_string()),
        "keep-all" => CSSValue::Keyword("keep-all".to_string()),
        "break-word" => CSSValue::Keyword("break-word".to_string()),
        other => CSSValue::Keyword(other.to_string()),
    }
}

/// overflow-wrap: 溢出换行
/// 值: normal | anywhere | break-word
pub fn parse_overflow_wrap(value: &str) -> CSSValue {
    match value.trim() {
        "normal" => CSSValue::Keyword("normal".to_string()),
        "anywhere" => CSSValue::Keyword("anywhere".to_string()),
        "break-word" => CSSValue::Keyword("break-word".to_string()),
        other => CSSValue::Keyword(other.to_string()),
    }
}

// ============================================================
//  变换属性 — 3D
// ============================================================

/// transform-style: 3D 空间上下文
/// 值: flat | preserve-3d
pub fn parse_transform_style(value: &str) -> CSSValue {
    match value.trim() {
        "preserve-3d" => CSSValue::Keyword("preserve-3d".to_string()),
        _ => CSSValue::Keyword("flat".to_string()),
    }
}

/// perspective: 3D 透视距离
/// 值: none | <length>
pub fn parse_perspective(value: &str) -> CSSValue {
    let value = value.trim();
    if value == "none" {
        return CSSValue::Keyword("none".to_string());
    }
    crate::values::parse_css_value("perspective", value)
}

/// perspective-origin: 透视原点
/// 值: <position>
pub fn parse_perspective_origin(value: &str) -> CSSValue {
    CSSValue::Keyword(value.trim().to_string())
}

/// backface-visibility: 背面可见性
/// 值: visible | hidden
pub fn parse_backface_visibility(value: &str) -> CSSValue {
    match value.trim() {
        "hidden" => CSSValue::Keyword("hidden".to_string()),
        _ => CSSValue::Keyword("visible".to_string()),
    }
}

// ============================================================
//  交互属性
// ============================================================

/// touch-action: 触控行为控制（移动端关键）
/// 值: auto | none | pan-x | pan-y | pinch-zoom | manipulation
pub fn parse_touch_action(value: &str) -> CSSValue {
    match value.trim() {
        "auto" => CSSValue::Keyword("auto".to_string()),
        "none" => CSSValue::Keyword("none".to_string()),
        "pan-x" => CSSValue::Keyword("pan-x".to_string()),
        "pan-y" => CSSValue::Keyword("pan-y".to_string()),
        "pinch-zoom" => CSSValue::Keyword("pinch-zoom".to_string()),
        "manipulation" => CSSValue::Keyword("manipulation".to_string()),
        other => CSSValue::Keyword(other.to_string()),
    }
}

/// will-change: GPU 合成层提示
/// 值: auto | transform | opacity | scroll-position | contents | <custom>
pub fn parse_will_change(value: &str) -> CSSValue {
    let value = value.trim();
    if value == "auto" {
        return CSSValue::Keyword("auto".to_string());
    }
    CSSValue::Keyword(value.to_string())
}

// ============================================================
//  Phase 3 新增属性的解析入口
// ============================================================

/// Phase 3 属性解析分发
pub fn parse_phase3_property(property: &str, value: &str) -> Option<CSSValue> {
    match property.to_lowercase().as_str() {
        "aspect-ratio" => Some(parse_aspect_ratio(value)),
        "contain" => Some(parse_contain(value)),
        "content-visibility" => Some(parse_content_visibility(value)),
        "outline-offset" => Some(parse_outline_offset(value)),
        "font-variant" => Some(parse_font_variant(value)),
        "font-stretch" => Some(parse_font_stretch(value)),
        "word-break" => Some(parse_word_break(value)),
        "overflow-wrap" => Some(parse_overflow_wrap(value)),
        "transform-style" => Some(parse_transform_style(value)),
        "perspective" => Some(parse_perspective(value)),
        "perspective-origin" => Some(parse_perspective_origin(value)),
        "backface-visibility" => Some(parse_backface_visibility(value)),
        "touch-action" => Some(parse_touch_action(value)),
        "will-change" => Some(parse_will_change(value)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aspect_ratio_auto() {
        let v = parse_aspect_ratio("auto");
        assert_eq!(v, CSSValue::Keyword("auto".to_string()));
    }

    #[test]
    fn test_aspect_ratio_fraction() {
        let v = parse_aspect_ratio("16/9");
        assert_eq!(v, CSSValue::Composite(vec![
            CSSValue::Number(16.0),
            CSSValue::Number(9.0),
        ]));
    }

    #[test]
    fn test_touch_action() {
        let v = parse_touch_action("none");
        assert_eq!(v, CSSValue::Keyword("none".to_string()));
    }

    #[test]
    fn test_will_change_auto() {
        let v = parse_will_change("auto");
        assert_eq!(v, CSSValue::Keyword("auto".to_string()));
    }
}
