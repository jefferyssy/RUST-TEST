//! DomRegistry — 扁平化 DOM 全局注册表。
//!
//! 运行时作为 `document` 全局对象使用，所有 DOM 操作通过 NodeId + &DomRegistry 完成。
//! 对标浏览器 `document` 对象。

use std::collections::{HashMap, HashSet};

use super::{CssStyleSheet, DomNode, NodeId, Rect, StyleSheetList, VNode};

/// DOM 全局注册表（即运行时 `document` 对象）。
///
/// 所有节点存储在 `allNodes: HashMap<NodeId, DomNode>` 中，
/// 通过 `idMap` / `classMap` / `tagMap` 反向索引加速选择器查询。
#[derive(Debug, Clone)]
pub struct DomRegistry {
    /// 所有节点（扁平存储）
    pub allNodes: HashMap<NodeId, DomNode>,
    /// id → NodeId 反向索引
    pub idMap: HashMap<String, NodeId>,
    /// class → NodeId[] 反向索引
    pub classMap: HashMap<String, Vec<NodeId>>,
    /// tag → NodeId[] 反向索引
    pub tagMap: HashMap<String, Vec<NodeId>>,
    /// 下一个分配的 ID
    pub nextId: u64,
    /// 样式脏节点集合
    pub styleDirtySet: HashSet<NodeId>,
    /// 绘制脏矩形列表
    pub paintDirtyRects: Vec<Rect>,
    /// document.styleSheets — CSSOM StyleSheetList
    pub styleSheets: StyleSheetList,
    /// body 根节点 ID
    pub bodyId: Option<NodeId>,
}

impl DomRegistry {
    /// 创建空的注册表。
    pub fn new() -> Self {
        Self {
            allNodes: HashMap::new(),
            idMap: HashMap::new(),
            classMap: HashMap::new(),
            tagMap: HashMap::new(),
            nextId: 0,
            styleDirtySet: HashSet::new(),
            paintDirtyRects: Vec::new(),
            styleSheets: StyleSheetList::new(),
            bodyId: None,
        }
    }

    /// 加载 VNode 树到 DOM，返回 body NodeId。
    pub fn loadVnode(&mut self, vnode: VNode) -> NodeId {
        let body_id = self.vnodeToDom(&vnode);
        self.bodyId = Some(body_id);
        body_id
    }

    /// 添加样式表到 document.styleSheets。
    pub fn addStyleSheet(&mut self, sheet: CssStyleSheet) {
        self.styleSheets.push(sheet);
    }

    // ── 内部：分配 ID ──

    fn alloc_id(&mut self) -> NodeId {
        let id = self.nextId;
        self.nextId += 1;
        NodeId::new(id)
    }

    // ── document 全局方法（对标浏览器 document.xxx）──

    /// 创建元素节点（对标 `document.createElement(tagName)`）。
    pub fn createElement(&mut self, tagName: &str) -> NodeId {
        let id = self.alloc_id();
        let node = DomNode::new_element(id, tagName.to_string());
        let tag_lower = tagName.to_lowercase();

        self.tagMap
            .entry(tag_lower.clone())
            .or_default()
            .push(id);
        self.allNodes.insert(id, node);
        id
    }

    /// 创建文本节点（对标 `document.createTextNode(data)`）。
    pub fn createTextNode(&mut self, data: &str) -> NodeId {
        let id = self.alloc_id();
        let mut node = DomNode::new_text(id);
        // 文本内容存入 attrs
        node.attrs.push(("__text".into(), data.to_string()));
        self.allNodes.insert(id, node);
        id
    }

    /// 按 ID 查找（对标 `document.getElementById(id)`）。
    pub fn getElementById(&self, id: &str) -> Option<NodeId> {
        self.idMap.get(id).copied()
    }

    /// 按 class 查找（对标 `document.getElementsByClassName(class)`）。
    pub fn getElementsByClassName(&self, class: &str) -> Vec<NodeId> {
        self.classMap.get(class).cloned().unwrap_or_default()
    }

    /// 按 tag 查找（对标 `document.getElementsByTagName(tag)`）。
    pub fn getElementsByTagName(&self, tag: &str) -> Vec<NodeId> {
        self.tagMap.get(&tag.to_lowercase()).cloned().unwrap_or_default()
    }

    /// 选择器查询（对标 `document.querySelector(selector)`）。
    /// 从 body 根开始搜索。
    pub fn querySelector(&self, selector: &str) -> Option<NodeId> {
        // 查找 body 节点（第一个无 parent 的元素节点），从 body 开始搜索
        let body = self.allNodes.values().find(|n| n.parentNode.is_none() && n.is_element())?;
        self.querySelector_from(body.nodeId, selector)
    }

    /// 选择器全量查询（对标 `document.querySelectorAll(selector)`）。
    pub fn querySelectorAll(&self, selector: &str) -> Vec<NodeId> {
        let mut results = Vec::new();
        if let Some(body) = self.allNodes.values().find(|n| n.parentNode.is_none() && n.is_element()) {
            self.querySelectorAll_from(body.nodeId, selector, &mut results);
        }
        results
    }

    /// 设置属性（对标 `element.setAttribute(name, value)`）。
    pub fn setAttribute(&mut self, nodeId: NodeId, name: &str, value: &str) {
        let is_id = name == "id";
        let is_class = name == "class";

        // 先更新节点内的属性值（避免同时持有 &mut node 和 &mut self）
        if let Some(node) = self.allNodes.get_mut(&nodeId) {
            if let Some((_, existing)) = node.attrs.iter_mut().find(|(k, _)| k == name) {
                *existing = value.to_string();
            } else {
                node.attrs.push((name.to_string(), value.to_string()));
            }

            if is_id {
                if let Some(old_id) = &node.id {
                    self.idMap.remove(old_id);
                }
                node.id = Some(value.to_string());
                self.idMap.insert(value.to_string(), nodeId);
            }

            // 特殊处理：style 属性 → 更新 node.style（cascade 从此字段读取内联样式）
            if name == "style" {
                node.style = value.to_string();
            }

            if !node.styleDirty {
                node.styleDirty = true;
                self.styleDirtySet.insert(nodeId);
            }
        }

        // class 更新需要独立 &mut self（上面已 drop）
        if is_class {
            self.update_class_list(nodeId, value);
        }
    }

    /// 获取属性（对标 `element.getAttribute(name)`）。
    pub fn getAttribute(&self, nodeId: NodeId, name: &str) -> Option<&str> {
        self.allNodes
            .get(&nodeId)?
            .attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    /// 设置文本内容（对标 `element.textContent = value`）。
    pub fn setTextContent(&mut self, nodeId: NodeId, text: &str) {
        if let Some(node) = self.allNodes.get_mut(&nodeId) {
            // 清空子节点（文本节点替换子节点）
            let child_ids: Vec<NodeId> = node.childNodes.drain(..).collect();
            for cid in child_ids {
                self.remove_node_recursive(cid);
            }
            // 创建新文本子节点
            let text_id = self.createTextNode(text);
            self.link_child(nodeId, text_id);
        }
    }

    /// 获取文本内容（对标 `element.textContent`）。
    pub fn getTextContent(&self, nodeId: NodeId) -> String {
        let mut text = String::new();
        self.collect_text_content(nodeId, &mut text);
        text
    }

    fn collect_text_content(&self, nodeId: NodeId, text: &mut String) {
        if let Some(node) = self.allNodes.get(&nodeId) {
            if node.is_text() {
                if let Some(t) = node.text_content() {
                    text.push_str(t);
                }
            }
            for &child_id in &node.childNodes {
                self.collect_text_content(child_id, text);
            }
        }
    }

    // ── 树操作 ──

    /// 追加子节点（对标 `parent.appendChild(child)`）。
    pub fn appendChild(&mut self, parentId: NodeId, childId: NodeId) {
        self.link_child(parentId, childId);
    }

    /// 移除子节点（对标 `parent.removeChild(child)`）。
    pub fn removeChild(&mut self, parentId: NodeId, childId: NodeId) {
        if let Some(parent) = self.allNodes.get_mut(&parentId) {
            parent.childNodes.retain(|&c| c != childId);
        }
        if let Some(child) = self.allNodes.get_mut(&childId) {
            child.parentNode = None;
        }
        self.remove_node_recursive(childId);
    }

    /// 在参考节点前插入（对标 `parent.insertBefore(newNode, refNode)`）。
    pub fn insertBefore(&mut self, parentId: NodeId, newId: NodeId, refId: NodeId) {
        if let Some(parent) = self.allNodes.get_mut(&parentId) {
            if let Some(pos) = parent.childNodes.iter().position(|&c| c == refId) {
                parent.childNodes.insert(pos, newId);
                if let Some(child) = self.allNodes.get_mut(&newId) {
                    child.parentNode = Some(parentId);
                }
            }
        }
    }

    // ── VNode → DomRegistry 转换 ──

    /// 将 VNode 树转换为 DomRegistry 节点（虚拟 → 真实 DOM）。
    pub fn vnodeToDom(&mut self, vnode: &VNode) -> NodeId {
        self.vnode_to_dom_inner(vnode, None)
    }

    fn vnode_to_dom_inner(&mut self, vnode: &VNode, parentId: Option<NodeId>) -> NodeId {
        match vnode {
            VNode::Element(el) => {
                let id = self.createElement(&el.tagName);
                if let Some(pid) = parentId {
                    self.link_child(pid, id);
                }
                // 设置属性
                if let Some(ref id_attr) = el.id {
                    self.setAttribute(id, "id", id_attr);
                }
                for class in &el.classList {
                    self.add_class(id, class);
                }
                if !el.style.is_empty() {
                    self.setAttribute(id, "style", &el.style);
                }
                for (key, val) in &el.attrs {
                    self.setAttribute(id, key, val);
                }
                // 设置 diffKey
                if let Some(ref key) = el.diffKey {
                    self.setAttribute(id, "__diffKey", key);
                }
                // 递归处理子节点
                for child in &el.childNodes {
                    self.vnode_to_dom_inner(child, Some(id));
                }
                id
            }
            VNode::Text(text) => {
                let id = self.createTextNode(text);
                if let Some(pid) = parentId {
                    self.link_child(pid, id);
                }
                id
            }
        }
    }

    // ── 脏标记 ──

    /// 递归标记整棵子树的 styleDirty。
    pub fn markAllDirty(&mut self, rootId: NodeId) {
        if let Some(node) = self.allNodes.get_mut(&rootId) {
            node.styleDirty = true;
            self.styleDirtySet.insert(rootId);
            let children: Vec<NodeId> = node.childNodes.clone();
            for child_id in children {
                self.markAllDirty(child_id);
            }
        }
    }

    /// 清空 dirty 集合（在 flushStyleDirty 后调用）。
    pub fn clearDirtySet(&mut self) {
        self.styleDirtySet.clear();
    }

    // ── 选择器查询（从指定节点搜索） ──

    /// 从指定节点向下搜索匹配选择器的一个节点。
    pub fn querySelector_from(&self, rootId: NodeId, selector: &str) -> Option<NodeId> {
        let node = self.allNodes.get(&rootId)?;
        // 先检查自身
        if self.element_matches(node, selector) {
            return Some(rootId);
        }
        // 递归子节点
        for &child_id in &node.childNodes {
            if let Some(found) = self.querySelector_from(child_id, selector) {
                return Some(found);
            }
        }
        None
    }

    /// 从指定节点向下搜索匹配选择器的所有节点。
    pub fn querySelectorAll_from(&self, rootId: NodeId, selector: &str, results: &mut Vec<NodeId>) {
        let node = self.allNodes.get(&rootId);
        if node.is_none() {
            return;
        }
        let node = node.unwrap();
        if self.element_matches(node, selector) {
            results.push(rootId);
        }
        for &child_id in &node.childNodes {
            self.querySelectorAll_from(child_id, selector, results);
        }
    }

    /// 检查元素是否匹配简单选择器。
    fn element_matches(&self, node: &DomNode, selector: &str) -> bool {
        let sel = selector.trim();

        if sel.is_empty() || sel == "*" {
            return node.is_element();
        }
        if sel.starts_with('#') {
            return node.id.as_deref() == Some(&sel[1..]);
        }
        if sel.starts_with('.') {
            return node.classList.contains(&sel[1..].to_string());
        }
        // tag 选择器
        return node.tagName.as_deref() == Some(&sel.to_lowercase());
    }

    // ── 内部辅助 ──

    /// 建立父子链接（自动维护索引）。
    fn link_child(&mut self, parentId: NodeId, childId: NodeId) {
        if let Some(parent) = self.allNodes.get_mut(&parentId) {
            parent.childNodes.push(childId);
        }
        if let Some(child) = self.allNodes.get_mut(&childId) {
            child.parentNode = Some(parentId);
        }
    }

    /// 递归移除节点（清理索引）。
    fn remove_node_recursive(&mut self, nodeId: NodeId) {
        if let Some(node) = self.allNodes.remove(&nodeId) {
            // 清理 idMap
            if let Some(ref id_str) = node.id {
                self.idMap.remove(id_str);
            }
            // 清理 classMap
            for class in &node.classList {
                if let Some(list) = self.classMap.get_mut(class) {
                    list.retain(|&id| id != nodeId);
                }
            }
            // 清理 tagMap
            if let Some(ref tag) = node.tagName {
                if let Some(list) = self.tagMap.get_mut(tag) {
                    list.retain(|&id| id != nodeId);
                }
            }
            // 清理 styleDirtySet
            self.styleDirtySet.remove(&nodeId);
            // 递归移除子节点
            for child_id in node.childNodes {
                self.remove_node_recursive(child_id);
            }
        }
    }

    /// 更新节点的 classList + classMap。
    fn update_class_list(&mut self, nodeId: NodeId, class_str: &str) {
        // 清理旧 classMap 条目
        let old_classes: Vec<String> = {
            if let Some(node) = self.allNodes.get(&nodeId) {
                node.classList.clone()
            } else {
                return;
            }
        };
        for c in &old_classes {
            if let Some(list) = self.classMap.get_mut(c) {
                list.retain(|&id| id != nodeId);
            }
        }

        // 设置新 classList
        let new_classes: Vec<String> = class_str
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        if let Some(node) = self.allNodes.get_mut(&nodeId) {
            node.classList = new_classes.clone();
        }
        for c in &new_classes {
            self.classMap.entry(c.clone()).or_default().push(nodeId);
        }
    }

    /// 添加单个 class。
    fn add_class(&mut self, nodeId: NodeId, class: &str) {
        if let Some(node) = self.allNodes.get_mut(&nodeId) {
            if !node.classList.contains(&class.to_string()) {
                node.classList.push(class.to_string());
            }
        }
        self.classMap
            .entry(class.to_string())
            .or_default()
            .push(nodeId);
    }
}

impl Default for DomRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "test/dom_registry.test.rs"]
mod tests;
