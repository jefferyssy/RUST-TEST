//! 绘制命令与 DisplayList
//!
//! PaintCommand 是渲染引擎消费的原子绘制操作。
//! DisplayList 是按 z-order 排序后的绘制命令列表。

use dom::{Color, Rect};

/// 像素坐标类型
pub type Pixel = f32;

/// 绘制命令 —— 渲染引擎处理的原子操作
///
/// 由 render_tree::builder 从 LayoutTree 生成
/// 被 render crate 消费并转换为 GPU 绘制调用
#[derive(Debug, Clone)]
pub enum PaintCommand {
    /// 填充矩形（背景色填充）
    FillRect {
        /// 矩形区域（视口坐标）
        rect: Rect<Pixel>,
        /// 填充颜色
        color: Color,
        /// 圆角半径（Phase 1+）
        radius: f32,
    },
    /// 绘制文本
    Text {
        text: String,
        font_size: f32,
        font_family: String,
        font_weight: u16,
        x: Pixel,
        y: Pixel,
        color: Color,
        /// 文本装饰（下划线/删除线等）
        decoration: TextDecoration,
    },
    /// 绘制边框
    Border {
        rect: Rect<Pixel>,
        widths: [f32; 4],
        colors: [Color; 4],
        /// 圆角半径
        radius: f32,
        /// 边框样式
        style: BorderStyle,
    },
    // Phase 1 新增
    /// 绘制阴影
    BoxShadow {
        rect: Rect<Pixel>,
        offset_x: f32,
        offset_y: f32,
        blur_radius: f32,
        spread_radius: f32,
        color: Color,
        inset: bool,
        /// 圆角半径（继承自元素的 border-radius）
        radius: f32,
    },
    /// 绘制图像
    Image {
        rect: Rect<Pixel>,
        /// 图像像素数据
        data: Vec<u8>,
        width: u32,
        height: u32,
        fit: ObjectFit,
    },
    /// 裁剪区域
    Clip {
        rect: Rect<Pixel>,
        commands: Vec<PaintCommand>,
    },
    /// 透明度分组
    Opacity {
        alpha: f32,
        commands: Vec<PaintCommand>,
    },
}

/// 边框样式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BorderStyle {
    None,
    Solid,
    Dashed,
    Dotted,
    Double,
    Groove,
    Ridge,
    Inset,
    Outset,
}

impl Default for BorderStyle {
    fn default() -> Self {
        BorderStyle::Solid
    }
}

/// 文本装饰
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextDecoration {
    None,
    Underline,
    Overline,
    LineThrough,
}

impl Default for TextDecoration {
    fn default() -> Self {
        TextDecoration::None
    }
}

/// 图像填充方式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ObjectFit {
    Fill,
    Contain,
    Cover,
    None,
    ScaleDown,
}

impl Default for ObjectFit {
    fn default() -> Self {
        ObjectFit::Fill
    }
}

/// 绘制命令列表 —— 按渲染顺序排列
///
/// P0-6: 使用 DFS 桶排序替代 O(N log N) 全排序。
/// 构建时按 z-order 层分桶，保证桶内 DFS 遍历序（即正确的 painter's order）。
#[derive(Debug)]
pub struct DisplayList {
    /// z-order 桶：索引 0=BoxShadow, 1=FillRect, 2=Image, 3=Border, 4=Text, 5=Clip, 6=Opacity
    buckets: [Vec<PaintCommand>; 7],
    /// P1-9: 单调递增的版本号，用于渲染器检测 DL 变更
    generation: u64,
}

impl Default for DisplayList {
    fn default() -> Self {
        Self::new()
    }
}

impl DisplayList {
    /// 创建空 DisplayList
    pub fn new() -> Self {
        const EMPTY_BUCKET: Vec<PaintCommand> = Vec::new();
        Self {
            buckets: [EMPTY_BUCKET; 7],
            generation: 0,
        }
    }

    /// P1-9: 获取 DisplayList 版本号
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// P1-9: 设置版本号（由 DisplayListBuilder 或 App 调用）
    pub fn set_generation(&mut self, gen: u64) {
        self.generation = gen;
    }

    /// P0-6: 按层添加绘制命令到对应桶
    pub fn push(&mut self, cmd: PaintCommand) {
        let bucket = z_bucket(&cmd);
        self.buckets[bucket].push(cmd);
    }

    /// P0-6: 按桶顺序合并，桶内保持 DFS 插入序
    pub fn sort_by_z_order(&mut self) {
        // 无需排序 — 桶内已按 DFS 序存储
        let total: usize = self.buckets.iter().map(|b| b.len()).sum();
        let mut merged = Vec::with_capacity(total);
        // 合并顺序：BoxShadow(-1等效bucket0), FillRect, Image, Border, Text, Clip, Opacity
        for bucket_idx in [0usize, 1, 2, 3, 4, 5, 6] {
            merged.append(&mut self.buckets[bucket_idx]);
        }
        self.buckets[0] = merged;
    }

    /// 获取排序后的命令列表（先调用 sort_by_z_order）
    pub fn commands(&self) -> &[PaintCommand] {
        &self.buckets[0]
    }

    /// 可变的命令切片（用于渲染后端修改）
    pub fn commands_mut(&mut self) -> &mut Vec<PaintCommand> {
        &mut self.buckets[0]
    }

    /// 获取命令总数（跨所有桶，无需 sort_by_z_order）
    pub fn len(&self) -> usize {
        self.buckets.iter().map(|b| b.len()).sum()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.buckets.iter().all(|b| b.is_empty())
    }

    /// 移除最后一条命令（从 bucket 0，假设已 sort）
    pub fn pop(&mut self) -> Option<PaintCommand> {
        self.buckets[0].pop()
    }

    /// P0-6: 直接追加到渲染列表（跳过桶排序，用于临时命令如光标闪烁）
    pub fn push_unsorted(&mut self, cmd: PaintCommand) {
        self.buckets[0].push(cmd);
    }

    /// 清空 DisplayList
    pub fn clear(&mut self) {
        for bucket in &mut self.buckets {
            bucket.clear();
        }
    }
}

/// P0-6: 获取绘制命令的 z-order 桶索引
fn z_bucket(cmd: &PaintCommand) -> usize {
    match cmd {
        PaintCommand::BoxShadow { .. } => 0, // 最底层：阴影
        PaintCommand::FillRect { .. } => 1,  // 背景
        PaintCommand::Image { .. } => 2,
        PaintCommand::Border { .. } => 3,
        PaintCommand::Text { .. } => 4,
        PaintCommand::Clip { .. } => 5,
        PaintCommand::Opacity { .. } => 6,   // 最顶层
    }
}

// Phase 1+: BorderRadius, BoxShadow, Image, Gradient, Clip, Opacity

#[cfg(test)]
#[path = "command.test.rs"]
mod tests;
