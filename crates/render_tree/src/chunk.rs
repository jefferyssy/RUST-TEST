//! P1-6: PaintChunk 分组 —— GPU 渲染批次
//!
//! 每个 PaintChunk 对应布局树中的一个元素节点，
//! 包含该节点的所有绘制命令及其共享的变换/裁剪/效果属性。
//! 渲染时以 Chunk 为单位提交 GPU，减少状态切换次数。
//!
//! 设计参考 Chromium cc::PaintChunk。

use dom::Rect;
use crate::command::PaintCommand;

/// 绘制块 —— 一组共享渲染属性的绘制命令
///
/// 每个布局元素节点产生一个 PaintChunk，
/// 块内命令共享 transform / clip / effect 属性。
#[derive(Debug)]
pub struct PaintChunk {
    /// 变换节点 ID（索引到 PropertyTrees::transforms，0 = identity）
    pub transform_id: u32,
    /// 裁剪节点 ID（索引到 PropertyTrees::clips，0 = 无裁剪）
    pub clip_id: u32,
    /// 效果节点 ID（索引到 PropertyTrees::effects，0 = 无效果）
    pub effect_id: u32,
    /// 该块的绘制命令列表
    pub commands: Vec<PaintCommand>,
    /// 元素矩形（用于脏区判断）
    pub rect: Rect<f32>,
}

impl PaintChunk {
    /// 创建新的 PaintChunk
    pub fn new(
        transform_id: u32,
        clip_id: u32,
        effect_id: u32,
        rect: Rect<f32>,
    ) -> Self {
        Self {
            transform_id,
            clip_id,
            effect_id,
            commands: Vec::new(),
            rect,
        }
    }

    /// 添加绘制命令到此块
    pub fn push(&mut self, cmd: PaintCommand) {
        self.commands.push(cmd);
    }

    /// 块中的命令数量
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// 块是否为空
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

/// PaintChunk 列表 —— 按渲染顺序排列
///
/// 与 DisplayList 互补：DisplayList 提供 z-order 桶排序的平面命令列表，
/// ChunkList 提供结构化的分组视图用于 GPU 批次提交。
#[derive(Debug)]
pub struct ChunkList {
    pub chunks: Vec<PaintChunk>,
    /// 关联的属性树
    pub property_trees: crate::property_trees::PropertyTrees,
}

impl ChunkList {
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            property_trees: super::property_trees::PropertyTrees::new(),
        }
    }

    /// 添加新块，返回块索引
    pub fn push_chunk(&mut self, chunk: PaintChunk) -> usize {
        let idx = self.chunks.len();
        self.chunks.push(chunk);
        idx
    }

    /// 从所有块中收集 DrawingCommand 到 DisplayList
    pub fn flatten_to_display_list(&self) -> crate::command::DisplayList {
        let mut list = crate::command::DisplayList::new();
        for chunk in &self.chunks {
            for cmd in &chunk.commands {
                list.push(cmd.clone());
            }
        }
        list.sort_by_z_order();
        list
    }

    /// 总块数
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }
}
