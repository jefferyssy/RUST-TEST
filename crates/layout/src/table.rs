//! Table 布局 —— 表格布局引擎
//!
//! Phase 1: 基础表格布局实现
//! Phase 2: colspan/rowspan, border-collapse, border-spacing
//! Table/TableRow/TableCell 三级结构

use std::collections::HashMap;

use crate::layout_box::{BoxType, LayoutBox};
use dom::Size;

/// 单元格跨度信息
#[derive(Debug, Clone, Default)]
struct CellSpan {
    colspan: u32,
    rowspan: u32,
}

/// 表格渲染上下文
struct TableContext {
    /// (row, col) → 该位置的单元格索引 (row_idx, col_idx)
    grid: HashMap<(usize, usize), (usize, usize)>,
    /// 列宽
    col_widths: Vec<f32>,
    /// 行高
    row_heights: Vec<f32>,
    /// 列数
    col_count: usize,
    /// 行数
    row_count: usize,
    /// 单元格间距
    border_spacing: f32,
    /// 是否合并边框
    border_collapse: bool,
}

/// Table 布局引擎
pub struct TableLayout;

impl TableLayout {
    /// 对 Table 容器执行布局
    ///
    /// 计算列宽（取每列最大宽度），行高（取每行最大高度），
    /// 然后按网格排列单元格。支持 colspan/rowspan。
    pub fn layout(&self, container: &mut LayoutBox, _viewport: Size<f32>) {
        let content_width = container.content_area().width;
        if content_width <= 0.0 || container.children.is_empty() {
            return;
        }

        // 构建表格上下文
        let ctx = self.build_context(container, content_width);
        if ctx.col_count == 0 || ctx.row_count == 0 {
            return;
        }

        // 排列单元格
        let spacing = if ctx.border_collapse { 0.0 } else { ctx.border_spacing };
        let start_x = container.rect.x + container.padding.left + container.border.left + spacing;
        let start_y = container.rect.y + container.padding.top + container.border.top + spacing;

        for ((row, col), &(row_i, col_i)) in &ctx.grid {
            let row = *row;
            let col = *col;

            let cell_x = start_x + ctx.col_widths[..col].iter().sum::<f32>()
                + spacing * col as f32;
            let cell_y = start_y + ctx.row_heights[..row].iter().sum::<f32>()
                + spacing * row as f32;

            // Span 宽度
            let span = self.get_cell_span(container, row_i, col_i);
            let span_end_col = (col + span.colspan as usize).min(ctx.col_count);
            let span_end_row = (row + span.rowspan as usize).min(ctx.row_count);

            let span_w: f32 = if span.colspan > 1 {
                ctx.col_widths[col..span_end_col].iter().sum::<f32>()
                    + spacing * (span.colspan - 1) as f32
            } else {
                ctx.col_widths[col]
            };
            let span_h: f32 = if span.rowspan > 1 {
                ctx.row_heights[row..span_end_row].iter().sum::<f32>()
                    + spacing * (span.rowspan - 1) as f32
            } else {
                ctx.row_heights[row]
            };

            // 定位单元格
            if col_i < container.children[row_i].children.len() {
                let cell = &mut container.children[row_i].children[col_i];
                cell.rect.x = cell_x;
                cell.rect.y = cell_y;
                cell.rect.width = span_w;
                cell.rect.height = span_h;
            }
        }
    }

    /// 构建表格网格上下文
    fn build_context(&self, container: &LayoutBox, content_width: f32) -> TableContext {
        let row_indices: Vec<usize> = container.children.iter()
            .enumerate()
            .filter(|(_, c)| matches!(c.box_type, BoxType::TableRow))
            .map(|(i, _)| i)
            .collect();

        if row_indices.is_empty() {
            return TableContext {
                grid: HashMap::new(),
                col_widths: Vec::new(),
                row_heights: Vec::new(),
                col_count: 0,
                row_count: 0,
                border_spacing: 2.0,
                border_collapse: false,
            };
        }

        // 计算网格（处理 colspan/rowspan）
        let mut grid: HashMap<(usize, usize), (usize, usize)> = HashMap::new();
        let mut col_count = 0usize;

        for (ri_idx, &row_i) in row_indices.iter().enumerate() {
            let row = &container.children[row_i];
            let mut col = 0usize;

            for (ci, _cell) in row.children.iter().enumerate() {
                let span = self.get_cell_span(container, row_i, ci);
                // 找到未被占据的位置
                while grid.contains_key(&(ri_idx, col)) {
                    col += 1;
                }
                grid.insert((ri_idx, col), (row_i, ci));
                // 标记 rowspan 占据的后续行
                for r in 1..span.rowspan as usize {
                    grid.entry((ri_idx + r, col))
                        .or_insert((row_i, ci));
                }
                for c in 0..span.colspan as usize {
                    if c > 0 {
                        grid.insert((ri_idx, col + c), (row_i, ci));
                    }
                }
                col += span.colspan as usize;
            }
            if col > col_count {
                col_count = col;
            }
        }

        let row_count = row_indices.len();
        let col_count = col_count.max(1);
        let mut col_widths = vec![0.0f32; col_count];
        let mut row_heights = vec![0.0f32; row_count];

        // 计算列宽
        for ((row, col), &(row_i, col_i)) in &grid {
            let span = self.get_cell_span(container, row_i, col_i);
            let row = &container.children[row_i];
            if col_i < row.children.len() && span.colspan == 1 {
                let cell = &row.children[col_i];
                let cell_w = cell.rect.width + cell.margin.left + cell.margin.right;
                if *col < col_count && cell_w > col_widths[*col] {
                    col_widths[*col] = cell_w;
                }
            }
        }

        // 分配剩余宽度
        let used: f32 = col_widths.iter().sum();
        if used < content_width && !col_widths.is_empty() {
            let extra = (content_width - used) / col_widths.len() as f32;
            for w in &mut col_widths {
                *w += extra;
            }
        }

        // 计算行高
        for (ri_idx, &row_i) in row_indices.iter().enumerate() {
            let row = &container.children[row_i];
            let mut max_h = 0.0f32;
            for cell in &row.children {
                let cell_h = cell.rect.height + cell.margin.top + cell.margin.bottom;
                if cell_h > max_h {
                    max_h = cell_h;
                }
            }
            row_heights[ri_idx] = max_h;
        }

        TableContext {
            grid,
            col_widths,
            row_heights,
            col_count,
            row_count,
            border_spacing: 2.0,
            border_collapse: false,
        }
    }

    /// 获取单元格的 colspan/rowspan
    fn get_cell_span(&self, container: &LayoutBox, row_i: usize, col_i: usize) -> CellSpan {
        let row = &container.children[row_i];
        if col_i >= row.children.len() {
            return CellSpan::default();
        }
        let cell = &row.children[col_i];

        let colspan = cell.computed_style.as_ref()
            .and_then(|s| s.get("colspan"))
            .and_then(|v| match v {
                style::values::CSSValue::Number(n) => Some(*n as u32),
                style::values::CSSValue::Keyword(s) => s.parse().ok(),
                _ => None,
            })
            .unwrap_or(1)
            .max(1);

        let rowspan = cell.computed_style.as_ref()
            .and_then(|s| s.get("rowspan"))
            .and_then(|v| match v {
                style::values::CSSValue::Number(n) => Some(*n as u32),
                style::values::CSSValue::Keyword(s) => s.parse().ok(),
                _ => None,
            })
            .unwrap_or(1)
            .max(1);

        CellSpan { colspan, rowspan }
    }
}
