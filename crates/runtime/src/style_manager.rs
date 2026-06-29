//! style_manager — CSS 样式引擎（对标 Chrome Blink StyleEngine）。

use crate::{ComputedStyle, CssRule, DomRegistry, NodeId, StyleSheetList};
use crate::cascade::cascade;
use crate::index::RuleIndex;
use crate::selector_match::SelectorMatchExt;

/// 样式表索引缓存。
#[derive(Debug, Clone)]
struct SheetIndex {
    /// 在 StyleSheetList 中的位置
    sheet_index: usize,
    /// 规则 → 候选加速索引
    rule_index: RuleIndex,
}

/// 运行时样式引擎（对标 Chrome Blink StyleEngine）。
///
/// 持有 `document.styleSheets` 的索引缓存。
/// `computeStyle` / `flushStyleDirty` 遍历所有活跃样式表。
pub struct StyleEngine {
    sheet_indices: Vec<SheetIndex>,
}

impl StyleEngine {
    pub fn new() -> Self {
        Self {
            sheet_indices: Vec::new(),
        }
    }

    /// 为所有样式表重建索引（在样式表变更后调用）。
    pub fn rebuildIndices(&mut self, style_sheets: &StyleSheetList) {
        self.sheet_indices.clear();
        for (i, sheet) in style_sheets.active_sheets().enumerate() {
            self.sheet_indices.push(SheetIndex {
                sheet_index: i,
                rule_index: RuleIndex::build(&sheet.cssRules),
            });
        }
    }

    /// 计算单个节点的最终样式。
    pub fn computeStyle(
        &self,
        style_sheets: &StyleSheetList,
        registry: &DomRegistry,
        node_id: NodeId,
    ) -> ComputedStyle {
        let node = match registry.allNodes.get(&node_id) {
            Some(n) => n,
            None => return ComputedStyle::default(),
        };

        let tag_name: Option<&str> = node.tagName.as_deref();
        let attrs: Vec<(String, String)> = node.attrs.clone();

        let mut candidates: Vec<(&CssRule, usize, usize)> = Vec::new();

        for si in &self.sheet_indices {
            let sheet = &style_sheets.item(si.sheet_index);
            let Some(sheet) = sheet else { continue };

            let rule_indices = si.rule_index.find_candidates(
                tag_name, &node.classList, node.id.as_deref(), &attrs,
            );
            for ri in rule_indices {
                if let Some(rule) = sheet.cssRules.get(ri) {
                    candidates.push((rule, ri, si.sheet_index));
                }
            }
        }

        // 从右向左完整匹配
        let mut matched: Vec<(&CssRule, usize, usize)> = Vec::new();
        for (rule, ri, si) in &candidates {
            if rule.selector.matches(registry, node_id) {
                matched.push((rule, *ri, *si));
            }
        }

        // 排序：先特异性，再 sheet 顺序，再 sheet 内规则顺序
        matched.sort_by(|(a, ai, a_si), (b, bi, b_si)| {
            a.selector.specificity
                .cmp(&b.selector.specificity)
                .then_with(|| a_si.cmp(b_si))
                .then_with(|| ai.cmp(bi))
        });

        cascade(
            &node.style,
            &matched.into_iter().map(|(r, _, _)| r.clone()).collect::<Vec<_>>(),
        )
    }

    /// 批量计算所有 styleDirty 节点的样式。
    pub fn flushStyleDirty(&self, registry: &mut DomRegistry) {
        let dirty_ids: Vec<NodeId> = registry.styleDirtySet.iter().copied().collect();

        // 阶段1：计算样式（只读借用，结束后释放）
        let computed_styles: Vec<(NodeId, ComputedStyle)> = {
            let style_sheets = &registry.styleSheets;
            dirty_ids
                .iter()
                .map(|&node_id| (node_id, self.computeStyle(style_sheets, registry, node_id)))
                .collect()
        };

        let mut affected_parents: Vec<(NodeId, NodeId)> = Vec::new();

        for (node_id, computed) in &computed_styles {
            let node = match registry.allNodes.get_mut(node_id) {
                Some(n) => n,
                None => continue,
            };

            let style_changed = node.computedStyle != *computed;
            node.computedStyle = computed.clone();
            node.styleDirty = false;

            if style_changed {
                node.layoutDirty = true;
                node.paintDirty = true;

                let dirty_rect = node.layoutRect;
                registry.paintDirtyRects.push(dirty_rect);

                if let Some(pid) = node.parentNode {
                    affected_parents.push((*node_id, pid));
                }
            }
        }

        for (_child_id, start_parent_id) in affected_parents {
            let mut current = Some(start_parent_id);
            while let Some(pid) = current {
                let p = match registry.allNodes.get_mut(&pid) {
                    Some(p) => p,
                    None => break,
                };
                if p.layoutDirty {
                    break;
                }
                p.layoutDirty = true;
                current = p.parentNode;
            }
        }

        registry.styleDirtySet.clear();
    }

    /// 统一执行管线（类似浏览器 DOMContentLoaded 后流程）。
    /// 重建索引 → 标记脏 → 计算样式 → 渲染 → on_ready 回调。
    pub fn run(
        &mut self,
        document: &mut DomRegistry,
        on_ready: impl FnOnce(&mut DomRegistry),
    ) {
        self.rebuildIndices(&document.styleSheets);
        if let Some(body_id) = document.bodyId {
            document.markAllDirty(body_id);
        }
        self.flushStyleDirty(document);
        on_ready(document);
    }
}

impl Default for StyleEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// 解析 CSS 字符串为 Vec<CssRule>（运行时用，如 JS insertRule / addDynamicSheet）。
pub fn parse_css(css: &str) -> Vec<CssRule> {
    let css = strip_comments(css);
    let mut rules = Vec::new();
    let mut remaining = css.as_str();

    while !remaining.is_empty() {
        remaining = remaining.trim_start();

        if remaining.is_empty() || remaining.starts_with('}') {
            break;
        }

        if let Some(brace_start) = remaining.find('{') {
            let selector_str = remaining[..brace_start].trim().to_string();
            remaining = &remaining[brace_start + 1..];

            if let Some(brace_end) = remaining.find('}') {
                let decl_block = &remaining[..brace_end];
                remaining = &remaining[brace_end + 1..];

                for sel in selector_str.split(',') {
                    let sel = sel.trim();
                    if sel.is_empty() {
                        continue;
                    }
                    if let Some(selector) = crate::ComplexSelector::parse(sel) {
                        let declarations = parse_declarations(decl_block);
                        if !declarations.is_empty() {
                            rules.push(CssRule {
                                selectorText: sel.to_string(),
                                selector,
                                style: declarations,
                            });
                        }
                    }
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }

    rules
}

fn strip_comments(css: &str) -> String {
    let mut result = String::new();
    let mut chars = css.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            while let Some(cc) = chars.next() {
                if cc == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

fn parse_declarations(block: &str) -> Vec<(String, String)> {
    let mut decls = Vec::new();
    for decl in block.split(';') {
        let decl = decl.trim();
        if decl.is_empty() {
            continue;
        }
        if let Some((prop, val)) = decl.split_once(':') {
            let prop = prop.trim().to_lowercase();
            let val = val.trim().to_string();
            if !prop.is_empty() && !val.is_empty() {
                decls.push((prop, val));
            }
        }
    }
    decls
}

#[cfg(test)]
#[path = "test/style_manager.test.rs"]
mod tests;
