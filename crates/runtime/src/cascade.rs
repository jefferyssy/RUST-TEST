//! cascade — CSS 样式层叠合并。

use std::collections::HashMap;

use crate::{ComputedStyle, CssRule};

/// 层叠合并：从匹配的 CSS 规则列表计算最终 ComputedStyle。
///
/// 优先级：内联 style > 高特异性选择器 > 低特异性选择器 > 默认值。
/// 同等特异性：后声明的规则覆盖先声明的规则。
///
/// `inlineStyle` 是节点已解析的内联样式声明（`element.style`），优先级最高。
/// `matchedRules` 已按规则表顺序排列（先声明的在前，后声明的在后）。
pub fn cascade(
    inlineStyle: &[(String, String)],
    matchedRules: &[CssRule],
) -> ComputedStyle {
    let mut style = ComputedStyle::default();
    // 临时 map：property -> (value, specificity_override)
    // specificity_override: None=来自选择器, Some(true)=来自内联
    let mut declarations: Vec<(String, String, u32, bool)> = Vec::new();

    // 1. 收集所有匹配规则的声明（按顺序，后面的覆盖前面的）
    for rule in matchedRules {
        for (prop, val) in &rule.style {
            declarations.push((
                prop.clone(),
                val.clone(),
                rule.selector.specificity.0 * 1000 + rule.selector.specificity.1 * 100 + rule.selector.specificity.2,
                false,
            ));
        }
    }

    // 2. 内联样式的声明（最高优先级，已结构化解析）
    for (prop, val) in inlineStyle {
        declarations.push((
            prop.clone(),
            val.clone(),
            u32::MAX, // 内联样式最高特异性
            true,
        ));
    }

    // 3. 按 property 分组，每组取最高特异性 + 最后声明的值
    let mut best: HashMap<String, (String, u32)> = HashMap::new();

    for (prop, val, specificity, _is_inline) in &declarations {
        let entry = best.entry(prop.clone()).or_insert((val.clone(), 0));
        if *specificity >= entry.1 {
            *entry = (val.clone(), *specificity);
        }
    }

    // 4. 应用最终声明到 ComputedStyle
    for (prop, (val, _)) in &best {
        apply_style_property(&mut style, prop, val);
    }

    style
}

/// 将单个 CSS 属性值应用到 ComputedStyle。
pub fn apply_style_property(style: &mut ComputedStyle, property: &str, value: &str) {
    let value = value.trim();

    match property.to_lowercase().as_str() {
        "background" | "background-color" | "bgcolor" => {
            style.bgColor = parse_color(value).unwrap_or(0);
        }
        "color" | "textcolor" => {
            style.textColor = parse_color(value).unwrap_or(0);
        }
        "width" => {
            style.width = parse_length(value);
        }
        "height" => {
            style.height = parse_length(value);
        }
        "padding" => {
            style.padding = parse_length(value).unwrap_or(0.0);
        }
        "margin" => {
            style.margin = parse_length(value).unwrap_or(0.0);
        }
        "font-size" | "fontsize" => {
            style.fontSize = parse_length(value).unwrap_or(0.0);
        }
        "font-family" | "fontfamily" => {
            style.fontFamily = Some(value.to_string());
        }
        "text-align" | "textalign" => {
            style.textAlign = Some(value.to_string());
        }
        "text-decoration" | "textdecoration" => {
            style.textDecoration = Some(value.to_string());
        }
        _ => {
            // 未知属性，一期忽略
        }
    }
}

/// 解析颜色值（#RRGGBB 或命名颜色）。
fn parse_color(value: &str) -> Option<u32> {
    let value = value.trim().to_lowercase();

    // 命名颜色
    match value.as_str() {
        "black" => return Some(0xFF000000),
        "white" => return Some(0xFFFFFFFF),
        "red" => return Some(0xFFFF0000),
        "green" => return Some(0xFF00FF00),
        "blue" => return Some(0xFF0000FF),
        "transparent" => return Some(0x00000000),
        _ => {}
    }

    // #RRGGBB 或 #RGB
    if value.starts_with('#') {
        let hex = &value[1..];
        return match hex.len() {
            6 => u32::from_str_radix(hex, 16).ok().map(|c| 0xFF000000 | c),
            3 => {
                // #RGB → #RRGGBB
                let r = u32::from_str_radix(&hex[0..1], 16).ok()?;
                let g = u32::from_str_radix(&hex[1..2], 16).ok()?;
                let b = u32::from_str_radix(&hex[2..3], 16).ok()?;
                Some(0xFF000000 | (r * 0x11) << 16 | (g * 0x11) << 8 | (b * 0x11))
            }
            _ => None,
        };
    }

    // rgb(r, g, b)
    if value.starts_with("rgb(") && value.ends_with(')') {
        let inner = &value[4..value.len() - 1];
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 3 {
            let r = parts[0].trim().parse::<u8>().ok()?;
            let g = parts[1].trim().parse::<u8>().ok()?;
            let b = parts[2].trim().parse::<u8>().ok()?;
            return Some(0xFF000000 | (r as u32) << 16 | (g as u32) << 8 | (b as u32));
        }
    }

    None
}

/// 解析长度值（px 单位）。
fn parse_length(value: &str) -> Option<f32> {
    let value = value.trim();

    // 纯数字（假设 px）
    if let Ok(v) = value.parse::<f32>() {
        return Some(v);
    }

    // 带 px 后缀
    if value.ends_with("px") {
        return value[..value.len() - 2].trim().parse::<f32>().ok();
    }

    None
}

#[cfg(test)]
#[path = "test/cascade.test.rs"]
mod tests;
