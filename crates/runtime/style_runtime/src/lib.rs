//! style_runtime — 运行时 CSS 匹配引擎（StyleEngine）。
//!
//! CSSOM 类型（CssRule, CssStyleSheet 等）移入 dom_flat。
//! 本 crate 负责选择器匹配 + 层叠计算。

#![allow(non_snake_case)]

pub mod selector_match;     // matches() impl on ComplexSelector
pub mod style_manager;      // StyleEngine + parse_css
pub mod cascade;
pub mod index;

// 从 dom_flat 重导出（向后兼容）
pub use dom_flat::{
    AttrOp, BasicSelector, Combinator, ComplexSelector,
    PseudoClass, PseudoElement, SelectSegment,
    Select, CssRule, CssStyleSheet, StyleSheetList, Specificity,
};

pub use selector_match::SelectorMatchExt;
pub use style_manager::{StyleEngine, parse_css};
pub use cascade::cascade;
pub use index::RuleIndex;
