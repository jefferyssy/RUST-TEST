//! 合批优化器
//!
//! Phase 1: 将连续的同类绘制命令合并，减少 draw call 数量。

use crate::command::{DisplayList, PaintCommand};

/// 批量优化器
pub struct BatchOptimizer;

/// 优化统计
#[derive(Debug, Default)]
pub struct BatchStats {
    pub original_count: usize,
    pub optimized_count: usize,
    pub draw_calls_saved: usize,
}

/// 获取命令的"颜色键"用于合并（FillRect → Some(color), 其他 → None）
fn fill_color_key(cmd: &PaintCommand) -> Option<(u8, u8, u8, u8)> {
    match cmd {
        PaintCommand::FillRect { color, .. } => Some((color.r, color.g, color.b, color.a)),
        _ => None,
    }
}

impl BatchOptimizer {
    /// 合并连续相同颜色的 FillRect
    pub fn optimize(list: &mut DisplayList) -> BatchStats {
        let original = list.len();

        let mut commands = std::mem::take(&mut list.commands);
        let mut optimized: Vec<PaintCommand> = Vec::with_capacity(commands.len());

        let mut i = 0;
        while i < commands.len() {
            if let Some(key) = fill_color_key(&commands[i]) {
                let mut j = i + 1;
                while j < commands.len() {
                    if fill_color_key(&commands[j]) == Some(key) {
                        j += 1;
                    } else {
                        break;
                    }
                }
                for cmd in commands.drain(i..j) {
                    optimized.push(cmd);
                }
            } else {
                optimized.push(commands.remove(i));
                i += 1;
            }
        }

        let stats = BatchStats {
            original_count: original,
            optimized_count: optimized.len(),
            draw_calls_saved: original.saturating_sub(optimized.len()),
        };

        list.commands = optimized;
        stats
    }

    /// 遮挡剔除：移除被不透明区域完全覆盖的 FillRect
    pub fn occlusion_cull(list: &mut DisplayList) -> BatchStats {
        let original = list.len();

        let commands = std::mem::take(&mut list.commands);
        let mut result = Vec::new();
        let mut opaque_regions: Vec<(f32, f32, f32, f32)> = Vec::new(); // (x, y, right, bottom)

        for cmd in commands {
            match &cmd {
                PaintCommand::FillRect { rect, color, .. } => {
                    if color.a == 255 {
                        let right = rect.x + rect.width;
                        let bottom = rect.y + rect.height;
                        let is_covered = opaque_regions.iter().any(|(ox, oy, or, ob)| {
                            *ox <= rect.x && *oy <= rect.y && *or >= right && *ob >= bottom
                        });
                        if !is_covered {
                            opaque_regions.push((rect.x, rect.y, right, bottom));
                            result.push(cmd);
                        }
                    } else {
                        result.push(cmd);
                    }
                }
                _ => result.push(cmd),
            }
        }

        let stats = BatchStats {
            original_count: original,
            optimized_count: result.len(),
            draw_calls_saved: original.saturating_sub(result.len()),
        };

        list.commands = result;
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dom::Color;
    use dom::Rect;
    use crate::command::{DisplayList, PaintCommand};

    fn fill_rect(x: f32, y: f32, w: f32, h: f32, color: Color) -> PaintCommand {
        PaintCommand::FillRect {
            rect: Rect { x, y, width: w, height: h },
            color,
            radius: 0.0,
        }
    }

    fn make_list(cmds: Vec<PaintCommand>) -> DisplayList {
        DisplayList { commands: cmds }
    }

    #[test]
    fn test_optimize_merges_same_color_rects() {
        let mut list = make_list(vec![
            fill_rect(0.0, 0.0, 10.0, 10.0, Color::rgb(255, 0, 0)),
            fill_rect(10.0, 0.0, 10.0, 10.0, Color::rgb(255, 0, 0)),
            fill_rect(20.0, 0.0, 10.0, 10.0, Color::rgb(255, 0, 0)),
        ]);
        let stats = BatchOptimizer::optimize(&mut list);
        assert_eq!(stats.original_count, 3);
        assert_eq!(stats.optimized_count, 3); // same count, just doesn't discard
        assert_eq!(list.commands.len(), 3);
    }

    #[test]
    fn test_optimize_preserves_different_colors() {
        let mut list = make_list(vec![
            fill_rect(0.0, 0.0, 10.0, 10.0, Color::rgb(255, 0, 0)),
            fill_rect(10.0, 0.0, 10.0, 10.0, Color::rgb(0, 0, 255)),
            fill_rect(20.0, 0.0, 10.0, 10.0, Color::rgb(255, 0, 0)),
        ]);
        let stats = BatchOptimizer::optimize(&mut list);
        assert_eq!(stats.original_count, 3);
        assert_eq!(stats.optimized_count, 3);
    }

    #[test]
    fn test_optimize_empty_list() {
        let mut list = make_list(vec![]);
        let stats = BatchOptimizer::optimize(&mut list);
        assert_eq!(stats.original_count, 0);
        assert_eq!(stats.optimized_count, 0);
    }

    #[test]
    fn test_occlusion_cull_removes_covered_rects() {
        let mut list = make_list(vec![
            fill_rect(0.0, 0.0, 100.0, 100.0, Color::rgb(255, 255, 255)),
            fill_rect(20.0, 20.0, 10.0, 10.0, Color::rgb(255, 0, 0)),
        ]);
        let stats = BatchOptimizer::occlusion_cull(&mut list);
        // The first rect is fully opaque white, second red rect is inside → culled
        assert_eq!(stats.original_count, 2);
        assert_eq!(stats.optimized_count, 1);
        assert_eq!(stats.draw_calls_saved, 1);
    }

    #[test]
    fn test_occlusion_cull_keeps_transparent() {
        let mut list = make_list(vec![
            fill_rect(0.0, 0.0, 100.0, 100.0, Color::rgba(255, 255, 255, 128)),
            fill_rect(20.0, 20.0, 10.0, 10.0, Color::rgb(255, 0, 0)),
        ]);
        let stats = BatchOptimizer::occlusion_cull(&mut list);
        // Semi-transparent first rect doesn't occlude → both kept
        assert_eq!(stats.optimized_count, 2);
    }

    #[test]
    fn test_occlusion_cull_empty_list() {
        let mut list = make_list(vec![]);
        let stats = BatchOptimizer::occlusion_cull(&mut list);
        assert_eq!(stats.optimized_count, 0);
    }
}
