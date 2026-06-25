//! query — NodeId 上的选择器查询方法。
//!
//! 对标浏览器 `element.querySelector()` / `element.querySelectorAll()` 等。
//! 所有方法第一个参数为 `&DomRegistry`（即 `&document`）。

use crate::{DomRegistry, NodeId};

/// NodeId 扩展：选择器查询（对标浏览器 element.xxx）。
pub trait DomQuery {
    /// 在子树中查找匹配选择器的第一个节点（对标 `element.querySelector(selector)`）。
    fn querySelector(self, document: &DomRegistry, selector: &str) -> Option<NodeId>;

    /// 在子树中查找匹配选择器的所有节点（对标 `element.querySelectorAll(selector)`）。
    fn querySelectorAll(self, document: &DomRegistry, selector: &str) -> Vec<NodeId>;

    /// 按 ID 在子树中查找（对标 `element.getElementById(id)`）。
    /// 注：浏览器中此方法仅在 document 上，此处为便利扩展。
    fn getElementById(self, document: &DomRegistry, id: &str) -> Option<NodeId>;

    /// 按 class 在子树中查找（对标 `element.getElementsByClassName(class)`）。
    fn getElementsByClassName(self, document: &DomRegistry, class: &str) -> Vec<NodeId>;

    /// 按 tag 在子树中查找（对标 `element.getElementsByTagName(tag)`）。
    fn getElementsByTagName(self, document: &DomRegistry, tag: &str) -> Vec<NodeId>;
}

impl DomQuery for NodeId {
    fn querySelector(self, document: &DomRegistry, selector: &str) -> Option<NodeId> {
        document.querySelector_from(self, selector)
    }

    fn querySelectorAll(self, document: &DomRegistry, selector: &str) -> Vec<NodeId> {
        let mut results = Vec::new();
        document.querySelectorAll_from(self, selector, &mut results);
        results
    }

    fn getElementById(self, document: &DomRegistry, id: &str) -> Option<NodeId> {
        // 在子树中 DFS 查找
        let node = document.allNodes.get(&self)?;
        if node.id.as_deref() == Some(id) && node.is_element() {
            return Some(self);
        }
        for &child_id in &node.childNodes {
            if let Some(found) = child_id.getElementById(document, id) {
                return Some(found);
            }
        }
        None
    }

    fn getElementsByClassName(self, document: &DomRegistry, class: &str) -> Vec<NodeId> {
        let mut results = Vec::new();
        self.collect_by_class(document, class, &mut results);
        results
    }

    fn getElementsByTagName(self, document: &DomRegistry, tag: &str) -> Vec<NodeId> {
        let mut results = Vec::new();
        self.collect_by_tag(document, &tag.to_lowercase(), &mut results);
        results
    }
}

impl NodeId {
    /// DFS 收集匹配 class 的节点。
    fn collect_by_class(self, document: &DomRegistry, class: &str, results: &mut Vec<NodeId>) {
        if let Some(node) = document.allNodes.get(&self) {
            if node.is_element() && node.classList.contains(&class.to_string()) {
                results.push(self);
            }
            for &child_id in &node.childNodes {
                child_id.collect_by_class(document, class, results);
            }
        }
    }

    /// DFS 收集匹配 tag 的节点。
    fn collect_by_tag(self, document: &DomRegistry, tag: &str, results: &mut Vec<NodeId>) {
        if let Some(node) = document.allNodes.get(&self) {
            if node.tagName.as_deref() == Some(tag) {
                results.push(self);
            }
            for &child_id in &node.childNodes {
                child_id.collect_by_tag(document, tag, results);
            }
        }
    }
}
