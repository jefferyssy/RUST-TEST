//! css_rule — CSSOM CSSStyleRule 类型定义。

use super::selector_match::ComplexSelector;
use super::select::Select;

/// 对应 CSSOM CSSStyleRule。
#[derive(Debug, Clone)]
#[allow(non_snake_case)]
pub struct CssRule {
    /// 原始选择器文本（如 ".container h1"）
    pub selectorText: String,
    /// 解析后的结构化选择器
    pub selector: ComplexSelector,
    /// 声明列表，对应 CSSStyleDeclaration（简化：Vec<(属性, 值)>）
    pub style: Vec<(String, String)>,
}

impl CssRule {
    /// 从 Select builder 创建规则。
    pub fn on(select: Select) -> Self {
        Self {
            selectorText: String::new(),
            selector: select.done(),
            style: Vec::new(),
        }
    }

    /// 链式添加声明。
    pub fn decl(mut self, prop: impl Into<String>, val: impl Into<String>) -> Self {
        self.style.push((prop.into(), val.into()));
        self
    }
}

/// 一行声明全部 CSS 规则。
///
/// ```ignore
/// let rules = css!(
///     "h1" { "color": "red", "font-size": "24px" },
///     ".container > button" { "background": "#007bff" },
/// );
/// ```
#[macro_export]
macro_rules! css {
    ( $( $sel:literal { $( $prop:literal : $val:literal ),* $(,)? } ),* $(,)? ) => {
        vec![
            $(
                $crate::CssRule {
                    selectorText: ($sel).into(),
                    selector: $crate::ComplexSelector::parse($sel)
                        .expect(concat!("invalid CSS selector: ", $sel)),
                    style: vec![
                        $(($prop.into(), ($val).into())),*
                    ],
                }
            ),*
        ]
    };
}
