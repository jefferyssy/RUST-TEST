//! P1-8: ConstraintSpace —— 布局约束空间
//!
//! 封装布局计算时节点的尺寸约束，替代直接的 Size<f32> 视口传递。
//! 参考 CSS Box Alignment / CSS Sizing 规范中的可用空间概念。

use dom::Size;

/// 约束空间 —— 布局算法可用的尺寸空间
///
/// 用于 `calculate_sizes` 和各个布局算法的 `layout` 方法。
/// 包含可用宽度/高度、最小/最大内容约束。
#[derive(Debug, Clone, Copy)]
pub struct ConstraintSpace {
    /// 可用宽度（父容器内容区域宽度）
    pub available_width: f32,
    /// 可用高度（父容器可提供的最大高度，f32::MAX 表示无限）
    pub available_height: f32,
    /// 最小内容尺寸（来自 min-width/min-height，无则 0.0）
    pub min_width: f32,
    pub min_height: f32,
    /// 最大内容尺寸（来自 max-width/max-height，无则 f32::MAX）
    pub max_width: f32,
    pub max_height: f32,
}

impl ConstraintSpace {
    /// 从可用宽高创建（假设无 min/max 约束）
    pub fn new(available_width: f32, available_height: f32) -> Self {
        Self {
            available_width,
            available_height,
            min_width: 0.0,
            min_height: 0.0,
            max_width: f32::MAX,
            max_height: f32::MAX,
        }
    }

    /// 从视口尺寸创建（根容器使用）
    pub fn from_viewport(viewport: Size<f32>) -> Self {
        Self::new(viewport.width, viewport.height)
    }

    /// 限制可用宽度（取 max_width 和 available 的较小值）
    pub fn constrained_width(&self) -> f32 {
        self.available_width.min(self.max_width)
    }

    /// 限制可用高度
    pub fn constrained_height(&self) -> f32 {
        self.available_height.min(self.max_height)
    }

    /// 子元素约束 —— 在父元素内容区域内派生
    pub fn for_child(&self) -> Self {
        Self {
            available_width: self.constrained_width(),
            available_height: self.constrained_height(),
            min_width: 0.0,
            min_height: 0.0,
            max_width: self.constrained_width(),
            max_height: self.constrained_height(),
        }
    }

    /// 设置最小尺寸
    pub fn with_min(mut self, min_w: f32, min_h: f32) -> Self {
        self.min_width = min_w;
        self.min_height = min_h;
        self
    }

    /// 设置最大尺寸
    pub fn with_max(mut self, max_w: f32, max_h: f32) -> Self {
        self.max_width = max_w;
        self.max_height = max_h;
        self
    }
}

impl Default for ConstraintSpace {
    fn default() -> Self {
        Self::new(f32::MAX, f32::MAX)
    }
}
