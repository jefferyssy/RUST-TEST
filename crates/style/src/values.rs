//! CSS 值类型系统
//!
//! 定义 CSS 属性值的类型化表示。

use dom::Color;

/// CSS 值类型
#[derive(Debug, Clone, PartialEq)]
pub enum CSSValue {
    /// 关键字（如 auto, none, inherit）
    Keyword(String),
    /// 长度值
    Length(f32, CSSUnit),
    /// 百分比
    Percentage(f32),
    /// 颜色
    Color(Color),
    /// 数值
    Number(f32),
    /// 字符串
    String(String),
    /// 复合值（如 border: 1px solid red）
    Composite(Vec<CSSValue>),
    /// 初始值标记
    Initial,
    /// calc(100% - 20px) 表达式 (Phase 1)
    Calc(Box<CalcValue>),
    /// transform 变换函数列表 (Phase 1)
    Transform(Vec<Transform>),
    /// 渐变 (Phase 1)
    Gradient(Box<Gradient>),
    /// 长度+百分比组合（用于 calc 中间结果）
    LengthPercentage(f32, f32, CSSUnit),
}

/// CSS 单位
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CSSUnit {
    Px,
    Em,
    Rem,
    /// 视口宽度 1% (Phase 1)
    Vw,
    /// 视口高度 1% (Phase 1)
    Vh,
    /// min(vw, vh) * 1% (Phase 1)
    Vmin,
    /// max(vw, vh) * 1% (Phase 1)
    Vmax,
    Percent,
    /// 角度单位：度 (Phase 1)
    Deg,
    /// 角度单位：弧度 (Phase 1)
    Rad,
    /// 角度单位：百分度 (Phase 1)
    Grad,
    /// 角度单位：圈数 (Phase 1)
    Turn,
    /// 时间单位：秒 (Phase 1)
    S,
    /// 时间单位：毫秒 (Phase 1)
    Ms,
    /// 分辨率单位：点/英寸 (Phase 1)
    Dpi,
    /// 分辨率单位：点/厘米 (Phase 1)
    Dpcm,
    /// 无单位数值
    None,
    // Phase 2+:
    // Fr,    // Grid 弹性系数
}

// ============================================================
//  Phase 1 新增类型
// ============================================================

/// Calc 表达式节点 —— 二进制树结构
#[derive(Debug, Clone, PartialEq)]
pub struct CalcValue {
    pub root: CalcNode,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CalcNode {
    /// 叶子：CSS 值
    Value(CSSValue),
    /// 加法
    Sum(Box<CalcNode>, Box<CalcNode>),
    /// 乘法
    Product(Box<CalcNode>, Box<CalcNode>),
    /// 取负
    Negate(Box<CalcNode>),
    /// 取倒数
    Invert(Box<CalcNode>),
}

/// Transform 变换函数
#[derive(Debug, Clone, PartialEq)]
pub enum Transform {
    /// 2D 矩阵
    Matrix([f32; 6]),
    /// 平移
    Translate(f32, f32),
    TranslateX(f32),
    TranslateY(f32),
    /// 缩放
    Scale(f32, f32),
    ScaleX(f32),
    ScaleY(f32),
    /// 旋转
    Rotate(f32),
    /// 倾斜
    Skew(f32, f32),
    SkewX(f32),
    SkewY(f32),
    // Phase 3: 3D 变换
    /// 3D 矩阵 (16 值)
    Matrix3d([f32; 16]),
    /// translateZ(z)
    TranslateZ(f32),
    /// translate3d(x, y, z)
    Translate3d(f32, f32, f32),
    /// rotateX(angle)
    RotateX(f32),
    /// rotateY(angle)
    RotateY(f32),
    /// rotateZ(angle) — 等价于 rotate(angle)
    RotateZ(f32),
    /// rotate3d(x, y, z, angle)
    Rotate3d(f32, f32, f32, f32),
    /// scaleZ(z)
    ScaleZ(f32),
    /// scale3d(x, y, z)
    Scale3d(f32, f32, f32),
    /// perspective(d)
    Perspective(f32),
}

/// 渐变
#[derive(Debug, Clone, PartialEq)]
pub struct Gradient {
    pub gradient_type: GradientType,
    pub direction: GradientDirection,
    pub stops: Vec<ColorStop>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GradientType {
    Linear,
    Radial,
    Conic,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GradientDirection {
    Angle(f32),
    Side(TopOrBottom, LeftOrRight),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TopOrBottom { Top, Bottom }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LeftOrRight { Left, Right }

/// 渐变色标
#[derive(Debug, Clone, PartialEq)]
pub struct ColorStop {
    pub color: Color,
    pub position: Option<f32>,
}

// ============================================================
//  Phase 3: Clip Path 形状函数
// ============================================================

/// Clip Path 形状 (Phase 3 新增)
#[derive(Debug, Clone, PartialEq)]
pub enum ClipPathShape {
    /// circle(r at x y)
    Circle { radius: f32, center: (f32, f32) },
    /// ellipse(rx ry at x y)
    Ellipse { rx: f32, ry: f32, center: (f32, f32) },
    /// polygon(x1 y1, x2 y2, ...)
    Polygon(Vec<(f32, f32)>),
    /// inset(top right bottom left round rx ry)
    Inset {
        top: f32,
        right: f32,
        bottom: f32,
        left: f32,
        border_radius: Option<(f32, f32)>,
    },
}

// ============================================================
//  Phase 3: Timing Function 扩展
// ============================================================

/// CSS 缓动函数 (Phase 3 新增 cubic-bezier / steps)
#[derive(Debug, Clone, PartialEq)]
pub enum TimingFunction {
    Ease,
    EaseIn,
    EaseOut,
    EaseInOut,
    Linear,
    /// cubic-bezier(x1, y1, x2, y2)
    CubicBezier(f32, f32, f32, f32),
    /// steps(n, start|end)
    Steps(u32, StepDirection),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StepDirection { Start, End }

/// 解析 CSS 属性值
pub fn parse_css_value(property: &str, value: &str) -> CSSValue {
    let _ = property;
    // 尝试颜色解析
    if let Some(color) = try_parse_color(value) {
        return CSSValue::Color(color);
    }
    // 尝试长度解析
    if let Some((length, unit)) = try_parse_length(value) {
        return CSSValue::Length(length, unit);
    }
    // 默认作为关键字
    CSSValue::Keyword(value.to_string())
}

/// 解析颜色值
pub fn parse_color(value: &str) -> Color {
    try_parse_color(value).unwrap_or(Color::BLACK)
}

/// 解析长度值
pub fn parse_length(value: &str) -> (f32, CSSUnit) {
    try_parse_length(value).unwrap_or((0.0, CSSUnit::Px))
}

/// 尝试解析颜色
fn try_parse_color(value: &str) -> Option<Color> {
    let value = value.trim();
    if let Some(hex) = value.strip_prefix('#') {
        return parse_hex_color(hex);
    }
    if value.starts_with("rgb") {
        return parse_rgb_color(value);
    }
    named_color(value)
}

/// 解析十六进制颜色 #xxx / #xxxxxx
fn parse_hex_color(hex: &str) -> Option<Color> {
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            Some(Color::rgb(r, g, b))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color::rgb(r, g, b))
        }
        _ => None,
    }
}

/// 解析 rgb() / rgba() 颜色
fn parse_rgb_color(value: &str) -> Option<Color> {
    let inner = value
        .trim_start_matches("rgba")
        .trim_start_matches("rgb")
        .trim_start_matches('(')
        .trim_end_matches(')')
        .trim();
    let parts: Vec<&str> = inner.split(|c| c == ',' || c == ' ').filter(|s| !s.is_empty()).collect();
    match parts.len() {
        3 => {
            let r = parts[0].trim().parse::<u8>().ok()?;
            let g = parts[1].trim().parse::<u8>().ok()?;
            let b = parts[2].trim().parse::<u8>().ok()?;
            Some(Color::rgb(r, g, b))
        }
        4 => {
            let r = parts[0].trim().parse::<u8>().ok()?;
            let g = parts[1].trim().parse::<u8>().ok()?;
            let b = parts[2].trim().parse::<u8>().ok()?;
            let a = (parts[3].trim().parse::<f32>().ok()? * 255.0) as u8;
            Some(Color::rgba(r, g, b, a))
        }
        _ => None,
    }
}

/// 命名颜色
fn named_color(name: &str) -> Option<Color> {
    match name.to_lowercase().as_str() {
        "black" => Some(Color::rgb(0, 0, 0)),
        "white" => Some(Color::rgb(255, 255, 255)),
        "red" => Some(Color::rgb(255, 0, 0)),
        "green" => Some(Color::rgb(0, 128, 0)),
        "blue" => Some(Color::rgb(0, 0, 255)),
        "yellow" => Some(Color::rgb(255, 255, 0)),
        "orange" => Some(Color::rgb(255, 165, 0)),
        "purple" => Some(Color::rgb(128, 0, 128)),
        "gray" | "grey" => Some(Color::rgb(128, 128, 128)),
        "transparent" => Some(Color::TRANSPARENT),
        _ => None,
    }
}

/// 尝试解析长度值
fn try_parse_length(value: &str) -> Option<(f32, CSSUnit)> {
    let value = value.trim();
    if value == "0" {
        return Some((0.0, CSSUnit::Px));
    }
    if let Some(rest) = value.strip_suffix("px") {
        let num: f32 = rest.trim().parse().ok()?;
        return Some((num, CSSUnit::Px));
    }
    if let Some(rest) = value.strip_suffix('%') {
        let num: f32 = rest.trim().parse().ok()?;
        return Some((num, CSSUnit::Percent));
    }
    if let Some(rest) = value.strip_suffix("rem") {
        let num: f32 = rest.trim().parse().ok()?;
        return Some((num, CSSUnit::Rem));
    }
    if let Some(rest) = value.strip_suffix("em") {
        let num: f32 = rest.trim().parse().ok()?;
        return Some((num, CSSUnit::Em));
    }
    None
}

// ============================================================
//  Phase 1 扩展解析：新单位
// ============================================================

/// 尝试解析长度值（扩展版本，支持 vw/vh/vmin/vmax）
fn try_parse_length_extended(value: &str) -> Option<(f32, CSSUnit)> {
    let value = value.trim();
    if value == "0" {
        return Some((0.0, CSSUnit::Px));
    }
    // Phase 1 新增单位（需要在 px/%/rem/em 之后检查）
    if let Some(rest) = value.strip_suffix("vmin") {
        let num: f32 = rest.trim().parse().ok()?;
        return Some((num, CSSUnit::Vmin));
    }
    if let Some(rest) = value.strip_suffix("vmax") {
        let num: f32 = rest.trim().parse().ok()?;
        return Some((num, CSSUnit::Vmax));
    }
    if let Some(rest) = value.strip_suffix("vh") {
        let num: f32 = rest.trim().parse().ok()?;
        return Some((num, CSSUnit::Vh));
    }
    if let Some(rest) = value.strip_suffix("vw") {
        let num: f32 = rest.trim().parse().ok()?;
        return Some((num, CSSUnit::Vw));
    }
    None
}

// ============================================================
//  Phase 1 新增解析函数
// ============================================================

/// 解析 transform 属性值
pub fn parse_transform(value: &str) -> Vec<Transform> {
    let value = value.trim();
    if value.is_empty() || value == "none" {
        return Vec::new();
    }
    let mut transforms = Vec::new();
    // 基础解析：按 ) 分割各变换函数
    for part in value.split(')') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (name, args_str) = if let Some(paren) = part.find('(') {
            (&part[..paren], part[paren + 1..].trim())
        } else {
            continue;
        };

        let args: Vec<f32> = args_str
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.trim().parse().ok())
            .collect();

        let transform = match name.trim().to_lowercase().as_str() {
            "matrix" if args.len() >= 6 => {
                let mut m = [0.0f32; 6];
                for (i, v) in args.iter().take(6).enumerate() {
                    m[i] = *v;
                }
                Transform::Matrix(m)
            }
            // Phase 3: 3D 变换
            "matrix3d" if args.len() >= 16 => {
                let mut m = [0.0f32; 16];
                for (i, v) in args.iter().take(16).enumerate() {
                    m[i] = *v;
                }
                Transform::Matrix3d(m)
            }
            "translate3d" if args.len() >= 3 => Transform::Translate3d(args[0], args[1], args[2]),
            "translatez" if !args.is_empty() => Transform::TranslateZ(args[0]),
            "translatex" if !args.is_empty() => Transform::TranslateX(args[0]),
            "translatey" if !args.is_empty() => Transform::TranslateY(args[0]),
            "translate" if args.len() >= 2 => Transform::Translate(args[0], args[1]),
            "scale3d" if args.len() >= 3 => Transform::Scale3d(args[0], args[1], args[2]),
            "scalez" if !args.is_empty() => Transform::ScaleZ(args[0]),
            "scalex" if !args.is_empty() => Transform::ScaleX(args[0]),
            "scaley" if !args.is_empty() => Transform::ScaleY(args[0]),
            "scale" if args.len() >= 2 => Transform::Scale(args[0], args[1]),
            "rotate3d" if args.len() >= 4 => Transform::Rotate3d(args[0], args[1], args[2], args[3]),
            "rotatex" if !args.is_empty() => Transform::RotateX(args[0]),
            "rotatey" if !args.is_empty() => Transform::RotateY(args[0]),
            "rotatez" if !args.is_empty() => Transform::RotateZ(args[0]),
            "rotate" if !args.is_empty() => Transform::Rotate(args[0]),
            "perspective" if !args.is_empty() => Transform::Perspective(args[0]),
            "skew" if args.len() >= 2 => Transform::Skew(args[0], args[1]),
            "skewx" if !args.is_empty() => Transform::SkewX(args[0]),
            "skewy" if !args.is_empty() => Transform::SkewY(args[0]),
            _ => continue,
        };
        transforms.push(transform);
    }
    transforms
}

/// 解析 calc() 表达式
pub fn parse_calc_expression(value: &str) -> CalcValue {
    let value = value.trim();
    // 去除 calc( 和 )
    let inner = value
        .strip_prefix("calc(")
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or(value);
    // Phase 1: 简单解析，存储为值节点
    CalcValue {
        root: CalcNode::Value(CSSValue::Keyword(inner.trim().to_string())),
    }
}

// ============================================================
//  Phase 3: CSS 数学函数 min() / max() / clamp()
// ============================================================

/// Phase 3: 解析 CSS 函数值
/// 新增支持: min(a, b, ...), max(a, b, ...), clamp(min, ideal, max)
pub fn parse_css_function(name: &str, args: &str) -> CSSValue {
    match name.trim().to_lowercase().as_str() {
        "min" => parse_min(args),
        "max" => parse_max(args),
        "clamp" => parse_clamp(args),
        _ => CSSValue::Keyword(format!("{}({})", name, args)),
    }
}

fn parse_min(args: &str) -> CSSValue {
    let _values = parse_comma_separated_values(args);
    CSSValue::Keyword(format!("min({})", args)) // Phase 3: 暂存为关键字，layout 阶段求值
}

fn parse_max(args: &str) -> CSSValue {
    let _values = parse_comma_separated_values(args);
    CSSValue::Keyword(format!("max({})", args))
}

fn parse_clamp(args: &str) -> CSSValue {
    let _values = parse_comma_separated_values(args);
    CSSValue::Keyword(format!("clamp({})", args))
}

fn parse_comma_separated_values(args: &str) -> Vec<CSSValue> {
    args.split(',')
        .map(|s| parse_css_value("", s.trim()))
        .collect()
}

// ============================================================
//  Phase 3: currentColor 解析
// ============================================================

/// Phase 3: 检查值是否为 currentColor 关键字
pub fn is_current_color(value: &str) -> bool {
    value.trim().eq_ignore_ascii_case("currentcolor")
}

// ============================================================
//  Phase 3: radial-gradient() 解析
// ============================================================

/// Phase 3: 解析 radial-gradient()
pub fn parse_radial_gradient(_value: &str) -> CSSValue {
    // Phase 3: 简单解析 radial-gradient 为 Gradient 值
    CSSValue::Gradient(Box::new(Gradient {
        gradient_type: GradientType::Radial,
        direction: GradientDirection::Angle(0.0),
        stops: Vec::new(),
    }))
}

// ============================================================
//  Phase 3: backdrop-filter 解析
// ============================================================

/// Phase 3: 解析 backdrop-filter
/// 解析逻辑同 filter 属性，但应用于元素背后的区域
pub fn parse_backdrop_filter(value: &str) -> CSSValue {
    CSSValue::Keyword(value.to_string())
}

// Phase 2+: parse_gradient, parse_filter, parse_animation

#[cfg(test)]
#[path = "values.test.rs"]
mod tests;
