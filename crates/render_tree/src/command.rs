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
#[derive(Debug, Default)]
pub struct DisplayList {
    pub(crate) commands: Vec<PaintCommand>,
}

impl DisplayList {
    /// 创建空 DisplayList
    pub fn new() -> Self {
        Self { commands: Vec::new() }
    }

    /// 添加绘制命令
    pub fn push(&mut self, cmd: PaintCommand) {
        self.commands.push(cmd);
    }

    /// 按 z-order 排序
    ///
    /// 排序规则：
    /// 1. FillRect（背景）最先
    /// 2. Image → Border → Text（内容层）
    /// 3. BoxShadow → Clip → Opacity（效果层）最后
    pub fn sort_by_z_order(&mut self) {
        self.commands.sort_by(|a, b| {
            fn layer(cmd: &PaintCommand) -> i32 {
                match cmd {
                    PaintCommand::FillRect { .. } => 0,
                    PaintCommand::Image { .. } => 1,
                    PaintCommand::Border { .. } => 2,
                    PaintCommand::Text { .. } => 3,
                    PaintCommand::BoxShadow { .. } => 4,
                    PaintCommand::Clip { .. } => 5,
                    PaintCommand::Opacity { .. } => 6,
                }
            }
            layer(a).cmp(&layer(b))
        });
    }

    /// 获取命令列表
    pub fn commands(&self) -> &[PaintCommand] {
        &self.commands
    }

    /// 清空 DisplayList
    pub fn clear(&mut self) {
        self.commands.clear();
    }

    /// 命令数量
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

// Phase 1+: BorderRadius, BoxShadow, Image, Gradient, Clip, Opacity

#[cfg(test)]
#[path = "command.test.rs"]
mod tests;
