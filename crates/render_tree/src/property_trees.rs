//! 属性树 —— transform / clip / effect 的树形存储
//!
//! 参考 Chromium cc::PropertyTrees 设计。
//! 绘制命令通过 u32 ID 引用树节点，而非内联属性值。
//! 优点：
//!   - 节点共享：多个绘制命令可引用同一变换/裁剪/效果节点
//!   - 增量更新：仅变更的节点需要更新
//!   - 内存紧凑：树结构比每条命令复制完整矩阵更高效

use dom::Rect;

/// 属性树集合 —— 包含三棵独立的属性树
#[derive(Debug, Clone)]
pub struct PropertyTrees {
    pub transforms: Vec<TransformNode>,
    pub clips: Vec<ClipNode>,
    pub effects: Vec<EffectNode>,
}

impl Default for PropertyTrees {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyTrees {
    pub fn new() -> Self {
        Self {
            // 索引 0 = 根节点（identity / 无裁剪 / 无效果）
            transforms: vec![TransformNode::root()],
            clips: vec![ClipNode::root()],
            effects: vec![EffectNode::root()],
        }
    }

    /// 插入变换节点，返回节点 ID
    pub fn push_transform(&mut self, parent_id: u32, translate_x: f32, translate_y: f32) -> u32 {
        let id = self.transforms.len() as u32;
        self.transforms.push(TransformNode {
            id,
            parent_id,
            translate_x,
            translate_y,
        });
        id
    }

    /// 插入裁剪节点，返回节点 ID
    pub fn push_clip(&mut self, parent_id: u32, rect: Rect<f32>) -> u32 {
        let id = self.clips.len() as u32;
        self.clips.push(ClipNode {
            id,
            parent_id,
            rect,
        });
        id
    }

    /// 插入效果节点，返回节点 ID
    pub fn push_effect(&mut self, parent_id: u32, opacity: f32) -> u32 {
        let id = self.effects.len() as u32;
        self.effects.push(EffectNode {
            id,
            parent_id,
            opacity,
        });
        id
    }

    /// 清空（保留根节点）
    pub fn clear(&mut self) {
        self.transforms.truncate(1);
        self.clips.truncate(1);
        self.effects.truncate(1);
    }
}

/// 变换节点
#[derive(Debug, Clone)]
pub struct TransformNode {
    pub id: u32,
    /// 父节点 ID（0 = 根节点，无变换）
    pub parent_id: u32,
    /// 平移（像素）
    pub translate_x: f32,
    pub translate_y: f32,
}

impl TransformNode {
    fn root() -> Self {
        Self {
            id: 0,
            parent_id: 0,
            translate_x: 0.0,
            translate_y: 0.0,
        }
    }

    /// 是否为根节点（无实际变换）
    pub fn is_root(&self) -> bool {
        self.id == 0
    }
}

/// 裁剪节点
#[derive(Debug, Clone)]
pub struct ClipNode {
    pub id: u32,
    /// 父节点 ID（0 = 无裁剪，全视口可见）
    pub parent_id: u32,
    /// 裁剪矩形（视口坐标）
    pub rect: Rect<f32>,
}

impl ClipNode {
    fn root() -> Self {
        Self {
            id: 0,
            parent_id: 0,
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
        }
    }

    pub fn is_root(&self) -> bool {
        self.id == 0
    }
}

/// 效果节点
#[derive(Debug, Clone)]
pub struct EffectNode {
    pub id: u32,
    /// 父节点 ID（0 = 无效果）
    pub parent_id: u32,
    /// 不透明度（0.0-1.0，1.0 = 完全不透明）
    pub opacity: f32,
}

impl EffectNode {
    fn root() -> Self {
        Self {
            id: 0,
            parent_id: 0,
            opacity: 1.0,
        }
    }

    pub fn is_root(&self) -> bool {
        self.id == 0
    }
}
