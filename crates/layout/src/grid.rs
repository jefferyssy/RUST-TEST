//! Grid 布局引擎 —— Phase 2
//!
//! 对应 W3C CSS Grid Layout Module Level 1。
//! 基础实现：固定列/行轨道 + 网格项定位。

use crate::layout_box::{BoxType, LayoutBox};
use dom::Size;

/// Grid 轨道定义
#[derive(Debug, Clone)]
pub enum GridTrack {
    /// 固定像素尺寸
    Fixed(f32),
    /// 弹性系数（fr 单位）
    Flex(f32),
    /// 自适应内容
    Auto,
    /// 百分比
    Percent(f32),
}

/// Grid 布局引擎
pub struct GridLayout;

impl GridLayout {
    /// 对 Grid 容器执行布局
    ///
    /// Phase 2: 基础实现 — 均匀分配列宽，逐行排列网格项。
    /// Phase 3+: 完整 grid-template-columns/rows, grid-area 定位。
    pub fn layout(&self, container: &mut LayoutBox, _viewport: Size<f32>) {
        let content_width = container.content_area().width;
        if content_width <= 0.0 || container.children.is_empty() {
            return;
        }

        // 收集 grid-template-columns 信息
        let (col_tracks, row_tracks) = Self::parse_grid_template(container);

        // 计算列宽
        let col_widths = Self::resolve_track_sizes(&col_tracks, content_width);

        // 计算行高
        let row_heights = Self::resolve_row_heights(container, &row_tracks, &col_widths);

        // 排列网格项
        let start_x = container.rect.x + container.padding.left + container.border.left;
        let start_y = container.rect.y + container.padding.top + container.border.top;

        let cols = col_widths.len().max(1);

        for (i, child) in container.children.iter_mut().enumerate() {
            if matches!(child.box_type, BoxType::GridItem) {
                let col = i % cols;
                let row = i / cols;

                child.rect.x = start_x + col_widths[..col].iter().sum::<f32>();
                child.rect.y = start_y + row_heights[..row.min(row_heights.len())].iter().sum::<f32>();

                if col < col_widths.len() {
                    child.rect.width = col_widths[col]
                        - child.margin.left - child.margin.right
                        - child.padding.left - child.padding.right;
                }

                if row < row_heights.len() {
                    child.rect.height = row_heights[row]
                        - child.margin.top - child.margin.bottom
                        - child.padding.top - child.padding.bottom;
                }
            }
        }
    }

    /// 解析 grid-template-columns / grid-template-rows
    fn parse_grid_template(container: &LayoutBox) -> (Vec<GridTrack>, Vec<GridTrack>) {
        let style = &container.computed_style;
        let mut cols = Vec::new();
        let mut rows = Vec::new();

        // Phase 2: 从 inline style 或 computed style 解析模板
        if let Some(s) = style {
            if let Some(template) = s.get("grid-template-columns") {
                cols = Self::parse_track_list(template);
            }
            if let Some(template) = s.get("grid-template-rows") {
                rows = Self::parse_track_list(template);
            }
        }

        // 默认：单列自适应
        if cols.is_empty() {
            cols.push(GridTrack::Flex(1.0));
        }
        if rows.is_empty() {
            rows.push(GridTrack::Auto);
        }

        (cols, rows)
    }

    fn parse_track_list(value: &style::values::CSSValue) -> Vec<GridTrack> {
        match value {
            style::values::CSSValue::Keyword(s) => {
                s.split_whitespace()
                    .filter_map(|part| Self::parse_single_track(part))
                    .collect()
            }
            _ => vec![GridTrack::Flex(1.0)],
        }
    }

    fn parse_single_track(s: &str) -> Option<GridTrack> {
        if s.ends_with("fr") {
            let num: f32 = s.trim_end_matches("fr").trim().parse().ok()?;
            Some(GridTrack::Flex(num))
        } else if s.ends_with("px") {
            let num: f32 = s.trim_end_matches("px").trim().parse().ok()?;
            Some(GridTrack::Fixed(num))
        } else if s.ends_with('%') {
            let num: f32 = s.trim_end_matches('%').trim().parse().ok()?;
            Some(GridTrack::Percent(num))
        } else if s == "auto" {
            Some(GridTrack::Auto)
        } else {
            None
        }
    }

    /// 将轨道定义解析为像素列宽
    fn resolve_track_sizes(tracks: &[GridTrack], available: f32) -> Vec<f32> {
        let mut sizes = vec![0.0f32; tracks.len()];
        let mut remaining = available;
        let mut flex_total = 0.0f32;

        // 第一遍：计算固定尺寸
        for (i, track) in tracks.iter().enumerate() {
            match track {
                GridTrack::Fixed(px) => {
                    sizes[i] = *px;
                    remaining -= px;
                }
                GridTrack::Percent(pct) => {
                    sizes[i] = available * pct / 100.0;
                    remaining -= sizes[i];
                }
                GridTrack::Flex(f) => {
                    flex_total += f;
                }
                GridTrack::Auto => {
                    sizes[i] = 0.0; // 将由内容决定
                }
            }
        }

        // 第二遍：分配弹性空间
        if flex_total > 0.0 && remaining > 0.0 {
            for (i, track) in tracks.iter().enumerate() {
                if let GridTrack::Flex(f) = track {
                    sizes[i] = remaining * f / flex_total;
                }
            }
        }

        // 确保非负
        for s in &mut sizes {
            if *s < 0.0 {
                *s = 0.0;
            }
        }

        sizes
    }

    /// 计算每行的最大高度
    fn resolve_row_heights(
        container: &LayoutBox,
        tracks: &[GridTrack],
        col_widths: &[f32],
    ) -> Vec<f32> {
        let cols = col_widths.len().max(1);
        let row_count = (container.children.len() + cols - 1) / cols;
        let mut heights = vec![0.0f32; row_count.max(1)];

        for (i, child) in container.children.iter().enumerate() {
            let row = i / cols;
            if row < heights.len() {
                let child_h = child.rect.height + child.margin.top + child.margin.bottom
                    + child.padding.top + child.padding.bottom;
                if child_h > heights[row] {
                    heights[row] = child_h;
                }
            }
        }

        // 应用显式行高
        for (i, track) in tracks.iter().enumerate() {
            if i < heights.len() {
                match track {
                    GridTrack::Fixed(px) => heights[i] = heights[i].max(*px),
                    GridTrack::Percent(_) => {} // 百分比相对于容器高度，Phase 3+
                    _ => {}
                }
            }
        }

        heights
    }
}
