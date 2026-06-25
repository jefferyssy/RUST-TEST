//! style_sheet — CSSOM StyleSheetList / CSSStyleSheet 类型定义。

use super::css_rule::CssRule;

/// 对应 CSSOM StyleSheetList — document.styleSheets
///
/// 编译期生成的 `create_style_sheet()` 返回单个 `CssStyleSheet`，
/// 在 main.rs 中 push 到此列表。运行时 JS 可通过此结构
/// 操作各个样式表（insertRule / deleteRule / disabled 等）。
#[derive(Debug, Clone)]
pub struct StyleSheetList {
    sheets: Vec<CssStyleSheet>,
}

impl StyleSheetList {
    pub fn new() -> Self { Self { sheets: Vec::new() } }

    // ── CSSOM 标准方法 ──

    pub fn length(&self) -> usize { self.sheets.len() }
    pub fn item(&self, index: usize) -> Option<&CssStyleSheet> { self.sheets.get(index) }
    pub fn item_mut(&mut self, index: usize) -> Option<&mut CssStyleSheet> { self.sheets.get_mut(index) }

    // ── 便利方法 ──

    pub fn push(&mut self, sheet: CssStyleSheet) { self.sheets.push(sheet); }

    pub fn iter(&self) -> impl Iterator<Item = &CssStyleSheet> { self.sheets.iter() }

    /// 遍历所有未被禁用的样式表（样式计算用）。
    pub fn active_sheets(&self) -> impl Iterator<Item = &CssStyleSheet> {
        self.sheets.iter().filter(|s| !s.disabled)
    }
}

impl Default for StyleSheetList {
    fn default() -> Self { Self::new() }
}

/// 对应 CSSOM CSSStyleSheet（含 StyleSheet 基类属性）。
///
/// `ownerNode` / `parentStyleSheet` / `ownerRule` 不适用：
/// 编译模型下 `<style>` 不进入 DOM，`@import` 编译期内联展开。
#[allow(non_snake_case)]
#[derive(Debug, Clone)]
pub struct CssStyleSheet {
    /// type: "text/css"（Rust 关键字冲突故用 sheetType）
    pub sheetType: String,
    /// <link href> 的 URL，inline 为 None
    pub href: Option<String>,
    /// 样式表标题
    pub title: Option<String>,
    /// 媒体查询，默认 "all"
    pub media: String,
    /// 是否禁用
    pub disabled: bool,
    /// 规则列表（对应 CSSRuleList）
    pub cssRules: Vec<CssRule>,
}

impl CssStyleSheet {
    /// 对应 CSSStyleSheet.insertRule(rule, index)
    pub fn insertRule(&mut self, rule: CssRule, index: usize) {
        self.cssRules.insert(index, rule);
    }

    /// 对应 CSSStyleSheet.deleteRule(index)
    pub fn deleteRule(&mut self, index: usize) {
        if index < self.cssRules.len() {
            self.cssRules.remove(index);
        }
    }
}
