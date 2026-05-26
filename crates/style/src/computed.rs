//! CoW ComputedStyle —— 基于 Arc 的写时复制样式存储
//!
//! 将 CSS 属性按类别分为 5 组，每组用 Arc<HashMap> 存储。
//! 继承时仅增加引用计数（零拷贝），修改时 Arc::make_mut 仅克隆受影响的组。

use std::collections::HashMap;
use std::sync::Arc;

use crate::property_id::PropertyId;
use crate::values::CSSValue;

const GROUP_COUNT: usize = 5;

/// 判断属性属于哪个分组 (0-4)
#[inline]
fn property_group(id: PropertyId) -> usize {
    use PropertyId::*;
    match id {
        // Group 0: 盒模型 + 尺寸
        Width | Height | MinWidth | MaxWidth | MinHeight | MaxHeight
        | Margin | MarginTop | MarginRight | MarginBottom | MarginLeft
        | Padding | PaddingTop | PaddingRight | PaddingBottom | PaddingLeft
        | Border | BorderTop | BorderRight | BorderBottom | BorderLeft
        | BoxSizing | AspectRatio => 0,

        // Group 1: 视觉 (颜色/背景/效果)
        Color | BackgroundColor | Background | Opacity | Visibility
        | BorderRadius | BoxShadow | Transform | Transition | Animation
        | ZIndex => 1,

        // Group 2: 文本排版
        FontSize | FontFamily | FontWeight | FontStyle | TextAlign
        | TextDecoration | WhiteSpace | LineHeight => 2,

        // Group 3: Flex/Grid/定位/浮动
        Display | Position | Float | Clear
        | FlexDirection | FlexWrap | JustifyContent | AlignItems | AlignContent
        | FlexGrow | FlexShrink | FlexBasis | AlignSelf | Order | Flex | Gap
        | GridTemplateColumns | GridTemplateRows | GridColumn | GridRow
        | Top | Right | Bottom | Left => 3,

        // Group 4: 溢出/表格/其他
        Overflow | OverflowX | OverflowY | Colspan | Rowspan => 4,
    }
}

/// 计算后的样式集合（CoW 实现）
///
/// 属性按类别分为 5 组，每组独立 Arc 存储：
/// - Group 0: 盒模型（width, margin, padding, border...）
/// - Group 1: 视觉（color, background, shadow, transform...）
/// - Group 2: 文本（font, text-align, line-height...）
/// - Group 3: 布局模式（display, flex, grid, position...）
/// - Group 4: 溢出/杂项（overflow, colspan...）
///
/// 继承时 Clone 仅增加 5 个 Arc 引用计数（~40 字节拷贝），
/// 修改时 Arc::make_mut 仅克隆受影响的分组。
#[derive(Debug)]
pub struct ComputedStyle {
    groups: [Arc<HashMap<PropertyId, CSSValue>>; GROUP_COUNT],
}

impl Clone for ComputedStyle {
    fn clone(&self) -> Self {
        Self {
            groups: [
                Arc::clone(&self.groups[0]),
                Arc::clone(&self.groups[1]),
                Arc::clone(&self.groups[2]),
                Arc::clone(&self.groups[3]),
                Arc::clone(&self.groups[4]),
            ],
        }
    }
}

impl PartialEq for ComputedStyle {
    fn eq(&self, other: &Self) -> bool {
        for i in 0..GROUP_COUNT {
            // 快速路径：同一个 Arc → 必然相等
            if Arc::ptr_eq(&self.groups[i], &other.groups[i]) {
                continue;
            }
            // 回退：逐元素比较
            if self.groups[i].as_ref() != other.groups[i].as_ref() {
                return false;
            }
        }
        true
    }
}

impl Eq for ComputedStyle {}

impl ComputedStyle {
    /// 创建空的 ComputedStyle（全部使用初始值）
    pub fn new() -> Self {
        Self {
            groups: [
                Arc::new(HashMap::new()),
                Arc::new(HashMap::new()),
                Arc::new(HashMap::new()),
                Arc::new(HashMap::new()),
                Arc::new(HashMap::new()),
            ],
        }
    }

    /// 获取属性值（字符串版，向后兼容）
    pub fn get(&self, name: &str) -> Option<&CSSValue> {
        PropertyId::from_str(name).and_then(|id| self.get_id(id))
    }

    /// 获取属性值（PropertyId 版，O(1) 分组查找）
    pub fn get_id(&self, id: PropertyId) -> Option<&CSSValue> {
        let group = property_group(id);
        self.groups[group].get(&id)
    }

    /// 获取属性值，不存在的属性返回 Initial
    pub fn get_or_initial(&self, name: &str) -> CSSValue {
        self.get(name).cloned().unwrap_or(CSSValue::Initial)
    }

    /// 设置属性值（字符串版，向后兼容）
    pub fn set(&mut self, name: &str, value: CSSValue) {
        if let Some(id) = PropertyId::from_str(name) {
            self.set_id(id, value);
        }
    }

    /// 设置属性值（PropertyId 版，触发 CoW）
    pub fn set_id(&mut self, id: PropertyId, value: CSSValue) {
        let group = property_group(id);
        Arc::make_mut(&mut self.groups[group]).insert(id, value);
    }

    /// 检查属性是否存在（PropertyId 版）
    pub fn contains_id(&self, id: PropertyId) -> bool {
        let group = property_group(id);
        self.groups[group].contains_key(&id)
    }

    /// 检查属性是否存在（字符串版，向后兼容）
    pub fn contains_key(&self, name: &str) -> bool {
        PropertyId::from_str(name)
            .map(|id| self.groups[property_group(id)].contains_key(&id))
            .unwrap_or(false)
    }

    /// 合并另一个样式（低优先级：仅添加不存在的属性）
    pub fn merge(&mut self, other: &ComputedStyle) {
        for group_idx in 0..GROUP_COUNT {
            let target = Arc::make_mut(&mut self.groups[group_idx]);
            for (&prop, val) in other.groups[group_idx].as_ref() {
                target.entry(prop).or_insert_with(|| val.clone());
            }
        }
    }

    /// 样式是否为空（无任何属性设置）
    pub fn is_empty(&self) -> bool {
        self.groups.iter().all(|g| g.is_empty())
    }

    /// 遍历所有属性（用于差异比较等）
    pub fn iter(&self) -> impl Iterator<Item = (&PropertyId, &CSSValue)> {
        self.groups
            .iter()
            .flat_map(|g| g.as_ref().iter())
    }
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
//  P1-4: StyleDifference — 增量样式更新
// ============================================================

/// 样式差异结果，用于增量布局/渲染
#[derive(Debug, Clone)]
pub struct StyleDiff {
    /// 所有变更的属性名（含 PropertyId）
    pub changed: Vec<PropertyId>,
    /// 是否影响布局（width/height/margin/padding/display/flex/position 等）
    pub layout_affecting: bool,
    /// 是否影响绘制（color/background/border/shadow/transform/opacity 等）
    pub paint_affecting: bool,
}

/// 比较两个 ComputedStyle，返回差异信息
///
/// P1-4: 用于增量渲染管线 ——
/// - `layout_affecting` 为 true 时触发 re-layout
/// - `paint_affecting` 为 true 时触发 re-paint
/// - 两者都为 false 时跳过重绘
pub fn diff_styles(old: &ComputedStyle, new: &ComputedStyle) -> StyleDiff {
    let mut changed = Vec::new();
    let mut layout_affecting = false;
    let mut paint_affecting = false;

    // 用 O(1) Arc 指针比较快速判断分组是否相同
    for group_idx in 0..GROUP_COUNT {
        // 快速路径：同一个 Arc → 必然相等
        if Arc::ptr_eq(&old.groups[group_idx], &new.groups[group_idx]) {
            continue;
        }

        let old_map = old.groups[group_idx].as_ref();
        let new_map = new.groups[group_idx].as_ref();

        // 收集旧属性中删除/修改的
        for (&prop, old_val) in old_map {
            match new_map.get(&prop) {
                Some(new_val) if old_val == new_val => {}
                _ => {
                    changed.push(prop);
                    if is_layout_property(prop) {
                        layout_affecting = true;
                    }
                    if is_paint_property(prop) {
                        paint_affecting = true;
                    }
                }
            }
        }

        // 收集新增的属性
        for &prop in new_map.keys() {
            if !old_map.contains_key(&prop) {
                changed.push(prop);
                if is_layout_property(prop) {
                    layout_affecting = true;
                }
                if is_paint_property(prop) {
                    paint_affecting = true;
                }
            }
        }
    }

    StyleDiff {
        changed,
        layout_affecting,
        paint_affecting,
    }
}

/// 判断属性是否影响布局（Group 0 盒模型 + Group 3 布局模式 + Group 4 溢出）
fn is_layout_property(id: PropertyId) -> bool {
    use PropertyId::*;
    matches!(id,
        Width | Height | MinWidth | MaxWidth | MinHeight | MaxHeight
        | Margin | MarginTop | MarginRight | MarginBottom | MarginLeft
        | Padding | PaddingTop | PaddingRight | PaddingBottom | PaddingLeft
        | Border | BorderTop | BorderRight | BorderBottom | BorderLeft
        | BoxSizing | AspectRatio
        | Display | Position | Float | Clear
        | FlexDirection | FlexWrap | JustifyContent | AlignItems | AlignContent
        | FlexGrow | FlexShrink | FlexBasis | AlignSelf | Order | Flex | Gap
        | GridTemplateColumns | GridTemplateRows | GridColumn | GridRow
        | Top | Right | Bottom | Left
        | Overflow | OverflowX | OverflowY
        | FontSize | LineHeight // 文本尺寸影响布局
    )
}

/// 判断属性是否影响绘制
fn is_paint_property(id: PropertyId) -> bool {
    use PropertyId::*;
    matches!(id,
        Color | BackgroundColor | Background | Opacity | Visibility
        | BorderRadius | BoxShadow | Transform
        | FontFamily | FontWeight | FontStyle
        | TextAlign | TextDecoration | WhiteSpace
        | ZIndex
    )
}
