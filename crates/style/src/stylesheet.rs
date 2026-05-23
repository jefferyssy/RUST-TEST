//! CSS 样式表解析
//!
//! 负责解析 CSS 文本为内部结构。

/// CSS 规则
#[derive(Debug, Clone)]
pub struct Rule {
    /// 选择器文本
    pub selectors: Vec<String>,
    /// 声明列表
    pub declarations: Vec<Declaration>,
}

/// CSS 声明（属性-值对）
#[derive(Debug, Clone)]
pub struct Declaration {
    pub property: String,
    pub value: String,
    pub important: bool,
}

/// CSS 样式表
#[derive(Debug, Clone)]
pub struct StyleSheet {
    /// 规则列表
    pub rules: Vec<Rule>,
    /// 来源 URL
    pub url: String,
}

impl StyleSheet {
    /// 创建空样式表
    pub fn new(url: &str) -> Self {
        Self {
            rules: Vec::new(),
            url: url.to_string(),
        }
    }
}

/// 解析完整 CSS 样式表
pub fn parse_stylesheet(_css: &str, url: &str) -> StyleSheet {
    // Phase 0: 使用 cssparser 解析
    // cssparser::parse_stylesheet(css, |result| { ... })
    StyleSheet::new(url)
}

/// 解析内联样式（style 属性值）
pub fn parse_inline_style(css: &str) -> Vec<Declaration> {
    // Phase 0: 简易解析，按 ; 分割
    let mut decls = Vec::new();
    for part in css.split(';') {
        let part = part.trim();
        if let Some((prop, val)) = part.split_once(':') {
            decls.push(Declaration {
                property: prop.trim().to_string(),
                value: val.trim().to_string(),
                important: false,
            });
        }
    }
    decls
}

// ============================================================
//  Phase 1: At-rules — @media, @keyframes, @font-face
// ============================================================

/// 媒体类型
#[derive(Debug, Clone, PartialEq)]
pub enum MediaType {
    All,
    Print,
    Screen,
    Custom(String),
}

/// 媒体特性
#[derive(Debug, Clone, PartialEq)]
pub enum MediaFeature {
    Width { min: Option<f32>, max: Option<f32> },
    Height { min: Option<f32>, max: Option<f32> },
    Orientation(String),
    AspectRatio { min: Option<f32>, max: Option<f32> },
    PrefersColorScheme(String),
    PrefersReducedMotion,
    Custom(String, String),
}

/// 媒体查询条件（已弃用，Phase 2 使用 MediaQuery）
#[derive(Debug, Clone)]
pub struct MediaCondition {
    pub media_type: MediaType,
    pub features: Vec<MediaFeature>,
    pub negated: bool,
}

/// 媒体查询
#[derive(Debug, Clone)]
pub struct MediaQuery {
    pub media_type: MediaType,
    pub features: Vec<MediaFeature>,
    pub rules: Vec<Rule>,
}

impl MediaQuery {
    pub fn new(media_type: MediaType) -> Self {
        Self {
            media_type,
            features: Vec::new(),
            rules: Vec::new(),
        }
    }
}

/// 关键帧选择器（百分比或 from/to 关键字）
#[derive(Debug, Clone)]
pub enum KeyframeSelector {
    From,
    To,
    Percent(f32),
}

/// 单个关键帧
#[derive(Debug, Clone)]
pub struct Keyframe {
    pub selector: KeyframeSelector,
    pub declarations: Vec<Declaration>,
    /// 解析后的属性值映射（Phase 2: 动画引擎使用）
    pub properties: std::collections::HashMap<String, crate::values::CSSValue>,
}

impl Keyframe {
    /// 获取关键帧所在的百分比位置 (0.0~1.0)
    pub fn percent(&self) -> f32 {
        match &self.selector {
            KeyframeSelector::From => 0.0,
            KeyframeSelector::To => 1.0,
            KeyframeSelector::Percent(p) => *p / 100.0,
        }
    }
}

/// @keyframes 动画规则
#[derive(Debug, Clone)]
pub struct KeyframesRule {
    pub name: String,
    pub keyframes: Vec<Keyframe>,
}

impl KeyframesRule {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            keyframes: Vec::new(),
        }
    }
}

/// @font-face 字体规则
#[derive(Debug, Clone)]
pub struct FontFaceRule {
    pub family: String,
    pub sources: Vec<String>,
    pub style: Option<String>,
    pub weight: Option<String>,
    pub display: Option<String>,
}

/// 解析媒体查询字符串
pub fn parse_media_query(_query: &str) -> Vec<MediaCondition> {
    // Phase 1: 使用 cssparser 解析
    // 返回解析后的条件列表
    Vec::new()
}

/// 解析 @keyframes 规则
pub fn parse_keyframes(_css: &str, _name: &str) -> KeyframesRule {
    // Phase 1: 使用 cssparser 解析帧
    KeyframesRule::new(_name)
}

/// 解析 @font-face 规则
pub fn parse_font_face(_css: &str) -> Option<FontFaceRule> {
    // Phase 1: 使用 cssparser 解析
    None
}

// ============================================================
//  Phase 3: @import 支持
// ============================================================

/// @import 规则 (Phase 3 新增)
#[derive(Debug, Clone)]
pub struct ImportRule {
    /// 导入路径
    pub url: String,
    /// 媒体查询条件（可选）
    pub media: Option<String>,
}

/// @import 错误类型
#[derive(Debug, Clone)]
pub enum ImportError {
    NotFound(String),
    CircularReference(String),
    ParseError(String),
}

impl StyleSheet {
    /// Phase 3: 支持 @import 递归加载
    /// 解析样式表中的 @import 规则并递归加载
    pub fn resolve_imports(&mut self, _base_path: &str) -> Result<(), ImportError> {
        // Phase 3: 从 CSS 文本中提取 @import 并递归加载
        // 当前作为占位实现，后续集成 cssparser 后完善
        Ok(())
    }
}

// Phase 2+: @supports, @container, @layer

#[cfg(test)]
#[path = "stylesheet.test.rs"]
mod tests;
