//! 命中检测 —— 根据屏幕坐标找到对应的 DOM 节点
//!
//! 用于将鼠标点击事件分发到正确的元素。
//! Phase 0: 基础实现。
//! Phase 1+: 在 WebWindow::handle_event 中集成。

use layout::layout_box::LayoutBox;
use dom::Rect;

/// 命中检测器 —— 根据屏幕坐标找到对应的 DOM 节点
///
/// 从根节点开始，逆序遍历子节点（z-order 高的优先）
/// 返回第一个包含该坐标的叶子节点
pub struct HitTester;

impl HitTester {
    /// 从布局树中查找坐标 (x, y) 对应的最深层节点
    pub fn hit_test<'a>(
        root: &'a LayoutBox,
        x: f32,
        y: f32,
    ) -> Option<&'a LayoutBox> {
        // 逆序遍历（后绘制在上层）
        for child in root.children.iter().rev() {
            if let Some(hit) = Self::hit_test(child, x, y) {
                return Some(hit);
            }
        }

        // 检查当前节点
        if Self::rect_contains(&root.rect, x, y) {
            Some(root)
        } else {
            None
        }
    }

    /// 检查矩形是否包含坐标点
    fn rect_contains(rect: &Rect<f32>, x: f32, y: f32) -> bool {
        x >= rect.x
            && x <= rect.x + rect.width
            && y >= rect.y
            && y <= rect.y + rect.height
    }

    /// 收集从根到目标节点的路径（用于事件冒泡）
    pub fn collect_bubble_path<'a>(
        root: &'a LayoutBox,
        target: &'a LayoutBox,
    ) -> Vec<&'a LayoutBox> {
        let mut path = Vec::new();
        Self::find_path(root, target, &mut path);
        path.reverse();
        path
    }

    /// 递归查找从根到目标的路径
    fn find_path<'a>(
        current: &'a LayoutBox,
        target: &'a LayoutBox,
        path: &mut Vec<&'a LayoutBox>,
    ) -> bool {
        if std::ptr::eq(current, target) {
            path.push(current);
            return true;
        }
        for child in &current.children {
            if Self::find_path(child, target, path) {
                path.push(current);
                return true;
            }
        }
        false
    }

    /// Phase 1: 多点触摸命中检测
    pub fn hit_test_multi<'a>(
        root: &'a LayoutBox,
        points: &[(f32, f32)],
    ) -> Vec<Option<&'a LayoutBox>> {
        points.iter()
            .map(|(x, y)| Self::hit_test(root, *x, *y))
            .collect()
    }

    /// Phase 1: 收集事件冒泡路径中所有可交互元素
    pub fn collect_interactive_path<'a>(
        root: &'a LayoutBox,
        x: f32,
        y: f32,
    ) -> Vec<&'a LayoutBox> {
        let mut path = Vec::new();
        Self::build_interactive_path(root, x, y, &mut path);
        path
    }

    fn build_interactive_path<'a>(
        current: &'a LayoutBox,
        x: f32,
        y: f32,
        path: &mut Vec<&'a LayoutBox>,
    ) {
        if !Self::rect_contains(&current.rect, x, y) {
            return;
        }
        path.push(current);
        for child in current.children.iter().rev() {
            Self::build_interactive_path(child, x, y, path);
        }
    }
}

// Phase 2+: pointer-events CSS 属性处理

#[cfg(test)]
mod tests {
    use super::*;
    use dom::Rect;
    use layout::layout_box::{BoxType, LayoutBox};

    fn make_box(x: f32, y: f32, w: f32, h: f32) -> LayoutBox {
        let mut b = LayoutBox::new(BoxType::Block, None);
        b.rect = Rect { x, y, width: w, height: h };
        b
    }

    fn make_box_with_children(x: f32, y: f32, w: f32, h: f32, children: Vec<LayoutBox>) -> LayoutBox {
        let mut b = LayoutBox::new(BoxType::Block, None);
        b.rect = Rect { x, y, width: w, height: h };
        b.children = children;
        b
    }

    #[test]
    fn test_rect_contains_point_inside() {
        let rect = Rect { x: 10.0, y: 10.0, width: 100.0, height: 50.0 };
        assert!(HitTester::rect_contains(&rect, 50.0, 30.0));
        assert!(HitTester::rect_contains(&rect, 10.0, 10.0));
        assert!(HitTester::rect_contains(&rect, 110.0, 60.0));
    }

    #[test]
    fn test_rect_contains_point_outside() {
        let rect = Rect { x: 10.0, y: 10.0, width: 100.0, height: 50.0 };
        assert!(!HitTester::rect_contains(&rect, 5.0, 30.0));
        assert!(!HitTester::rect_contains(&rect, 50.0, 5.0));
        assert!(!HitTester::rect_contains(&rect, 120.0, 30.0));
        assert!(!HitTester::rect_contains(&rect, 50.0, 70.0));
    }

    #[test]
    fn test_hit_test_root_only() {
        let root = make_box(0.0, 0.0, 800.0, 600.0);
        assert!(HitTester::hit_test(&root, 400.0, 300.0).is_some());
        assert!(HitTester::hit_test(&root, 900.0, 300.0).is_none());
    }

    #[test]
    fn test_hit_test_returns_deepest_child() {
        let child = make_box(50.0, 50.0, 100.0, 30.0);
        let root = make_box_with_children(0.0, 0.0, 800.0, 600.0, vec![child]);
        let hit = HitTester::hit_test(&root, 100.0, 65.0);
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().rect.width, 100.0);
    }

    #[test]
    fn test_hit_test_reverse_z_order() {
        let back = make_box(10.0, 10.0, 80.0, 80.0);
        let front = make_box(30.0, 30.0, 80.0, 80.0);
        let root = make_box_with_children(0.0, 0.0, 200.0, 200.0, vec![back, front]);
        let hit = HitTester::hit_test(&root, 50.0, 50.0);
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().rect.x, 30.0);
    }

    #[test]
    fn test_hit_test_multi_points() {
        let child = make_box(50.0, 50.0, 100.0, 30.0);
        let root = make_box_with_children(0.0, 0.0, 800.0, 600.0, vec![child]);
        let results = HitTester::hit_test_multi(&root, &[(75.0, 65.0), (900.0, 900.0)]);
        assert_eq!(results.len(), 2);
        assert!(results[0].is_some());
        assert!(results[1].is_none());
    }

    #[test]
    fn test_collect_bubble_path_depth() {
        let grandchild = make_box(60.0, 60.0, 20.0, 20.0);
        let child = make_box_with_children(50.0, 50.0, 100.0, 100.0, vec![grandchild]);
        let root = make_box_with_children(0.0, 0.0, 800.0, 600.0, vec![child]);
        let target = &root.children[0].children[0];
        let path = HitTester::collect_bubble_path(&root, target);
        assert_eq!(path.len(), 3);
    }

    #[test]
    fn test_collect_interactive_path_depth() {
        let grandchild = make_box(60.0, 60.0, 20.0, 20.0);
        let child = make_box_with_children(50.0, 50.0, 100.0, 100.0, vec![grandchild]);
        let root = make_box_with_children(0.0, 0.0, 800.0, 600.0, vec![child]);
        let path = HitTester::collect_interactive_path(&root, 70.0, 70.0);
        assert_eq!(path.len(), 3);
    }

    #[test]
    fn test_collect_interactive_path_outside() {
        let root = make_box(0.0, 0.0, 100.0, 100.0);
        let path = HitTester::collect_interactive_path(&root, 200.0, 200.0);
        assert!(path.is_empty());
    }
}