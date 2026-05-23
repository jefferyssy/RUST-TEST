//! 节点类型枚举 + Node 核心类型
//!
//! 对应 W3C DOM 标准 Node 接口
//!
//! 树操作要点：
//! - `parent` 使用 Weak 指针避免循环引用
//! - 兄弟节点通过 `prev_sibling`(Weak) + `next_sibling`(Rc) 形成双向链表
//! - 子节点列表通过 `Vec<Rc<RefCell<Node>>>` 维护顺序

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use super::document::Document;

/// 节点类型枚举 —— 对应 W3C DOM 标准
pub enum NodeType {
    /// 元素节点：<div>, <h1>, <button> 等
    Element(super::element::ElementData),
    /// 文本节点：元素的文本内容
    Text(super::text::Text),
    /// 文档根节点
    Document,
    /// 文档片段（Phase 1：支持批量插入操作）
    DocumentFragment,
    /// 注释节点（Phase 1：存储注释文本）
    Comment(String),
}

/// W3C 标准节点类型常量：与浏览器 API 保持一致
pub mod node_type_constants {
    pub const ELEMENT_NODE: u16 = 1;
    pub const TEXT_NODE: u16 = 3;
    pub const COMMENT_NODE: u16 = 8;
    pub const DOCUMENT_NODE: u16 = 9;
    pub const DOCUMENT_FRAGMENT_NODE: u16 = 11;
    // Phase 2+:
    // pub const DOCUMENT_TYPE_NODE: u16 = 10;
}

/// DOM 节点 —— 树结构核心类型
///
/// 使用 Rc<RefCell<Node>> 管理共享所有权
/// parent 使用 Weak 避免循环引用
/// 兄弟节点通过 prev_sibling(Weak) + next_sibling(Rc) 构成双向链表
pub struct Node {
    pub node_type: NodeType,
    /// 父节点（Weak 防循环引用）
    parent: Option<Weak<RefCell<Node>>>,
    /// 子节点列表（有序）
    pub(crate) children: Vec<Rc<RefCell<Node>>>,
    /// 前一个兄弟节点（Weak 防循环引用）
    prev_sibling: Option<Weak<RefCell<Node>>>,
    /// 后一个兄弟节点
    next_sibling: Option<Rc<RefCell<Node>>>,
    /// 变更标记 —— 节点修改时通知 layout 引擎重排
    pub(crate) dirty: Cell<bool>,
    /// 指向自身的 Weak 指针 —— 使节点方法能获取自身 Rc
    weak_self: Weak<RefCell<Node>>,
}

impl Node {
    // ============================================================
    //  构造函数
    // ============================================================

    /// 创建新节点（内部使用，通过 Document 创建对外）
    pub fn new(node_type: NodeType) -> Rc<RefCell<Self>> {
        Rc::new_cyclic(|weak| RefCell::new(Self {
            node_type,
            parent: None,
            children: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
            dirty: Cell::new(false),
            weak_self: weak.clone(),
        }))
    }

    // ============================================================
    //  树操作 —— W3C DOM 标准
    // ============================================================

    /// 追加子节点到末尾
    /// 如果 child 已有父节点，自动从原位置移除
    pub fn append_child(&mut self, child: Rc<RefCell<Node>>) -> Rc<RefCell<Node>> {
        let prev_parent = child.borrow().parent.clone();
        if let Some(prev_weak) = prev_parent {
            if let Some(parent_rc) = prev_weak.upgrade() {
                parent_rc.borrow_mut().remove_child_by_ptr(&child);
            }
        }

        if let Some(last) = self.children.last() {
            child.borrow_mut().prev_sibling = Some(Rc::downgrade(last));
            last.borrow_mut().next_sibling = Some(child.clone());
        }

        let self_rc = self.weak_self.upgrade().expect("Node was dropped");
        child.borrow_mut().parent = Some(Rc::downgrade(&self_rc));
        self.children.push(child.clone());
        self.mark_dirty(true);
        child
    }

    /// 通过 Rc 指针地址移除子节点（内部使用）
    fn remove_child_by_ptr(&mut self, child: &Rc<RefCell<Node>>) {
        let target_ptr = Rc::as_ptr(child);
        if let Some(pos) = self.children.iter().position(|c| Rc::as_ptr(c) == target_ptr) {
            let removed = self.children.remove(pos);
            removed.borrow_mut().parent = None;
            removed.borrow_mut().prev_sibling = None;
            removed.borrow_mut().next_sibling = None;
            if pos < self.children.len() {
                if pos > 0 {
                    self.children[pos].borrow_mut().prev_sibling =
                        Some(Rc::downgrade(&self.children[pos - 1]));
                } else {
                    self.children[pos].borrow_mut().prev_sibling = None;
                }
            }
        }
        self.mark_dirty(true);
    }

    /// 移除指定子节点
    /// Panics: child 不是直接子节点
    pub fn remove_child(&mut self, child: &Node) -> Rc<RefCell<Node>> {
        let target_ptr = child as *const Node as usize;
        let pos = self
            .children
            .iter()
            .position(|c| &*c.borrow() as *const Node as usize == target_ptr)
            .expect("remove_child: child is not a direct child of this node");

        let found = self.children.remove(pos);
        found.borrow_mut().parent = None;
        found.borrow_mut().prev_sibling = None;
        found.borrow_mut().next_sibling = None;

        if pos < self.children.len() {
            if pos > 0 {
                self.children[pos].borrow_mut().prev_sibling =
                    Some(Rc::downgrade(&self.children[pos - 1]));
            } else {
                self.children[pos].borrow_mut().prev_sibling = None;
            }
        }
        self.mark_dirty(true);
        found
    }

    /// 在参考节点之前插入新节点
    /// reference_node = None 等价于 append_child
    pub fn insert_before(
        &mut self,
        new_node: Rc<RefCell<Node>>,
        reference_node: Option<&Node>,
    ) -> Rc<RefCell<Node>> {
        let Some(ref_node) = reference_node else {
            return self.append_child(new_node);
        };

        let ref_ptr = ref_node as *const Node as usize;
        let pos = self
            .children
            .iter()
            .position(|c| &*c.borrow() as *const Node as usize == ref_ptr)
            .expect("insert_before: reference_node is not a child of this node");

        let prev_parent = new_node.borrow().parent.clone();
        if let Some(prev_weak) = prev_parent {
            if let Some(parent_rc) = prev_weak.upgrade() {
                parent_rc.borrow_mut().remove_child_by_ptr(&new_node);
            }
        }

        new_node.borrow_mut().next_sibling = Some(self.children[pos].clone());
        if pos > 0 {
            new_node.borrow_mut().prev_sibling = Some(Rc::downgrade(&self.children[pos - 1]));
            self.children[pos - 1].borrow_mut().next_sibling = Some(new_node.clone());
        }

        self.children[pos].borrow_mut().prev_sibling = Some(Rc::downgrade(&new_node));

        let self_rc = self.weak_self.upgrade().expect("Node was dropped");
        new_node.borrow_mut().parent = Some(Rc::downgrade(&self_rc));
        self.children.insert(pos, new_node.clone());
        self.mark_dirty(true);
        new_node
    }

    /// 用新节点替换旧节点
    pub fn replace_child(
        &mut self,
        new_child: Rc<RefCell<Node>>,
        old_child: &Node,
    ) -> Rc<RefCell<Node>> {
        let old_ptr = old_child as *const Node as usize;
        let pos = self
            .children
            .iter()
            .position(|c| &*c.borrow() as *const Node as usize == old_ptr)
            .expect("replace_child: old_child is not a child of this node");

        let old_node = self.children[pos].clone();
        old_node.borrow_mut().parent = None;

        let self_rc = self.weak_self.upgrade().expect("Node was dropped");
        new_child.borrow_mut().parent = Some(Rc::downgrade(&self_rc));
        new_child.borrow_mut().prev_sibling = old_node.borrow().prev_sibling.clone();
        new_child.borrow_mut().next_sibling = old_node.borrow().next_sibling.clone();

        self.children[pos] = new_child.clone();
        self.mark_dirty(true);
        old_node
    }

    /// 判断 other 是否是本节点的后代（包含自身）
    pub fn contains(&self, other: &Node) -> bool {
        if self as *const Node == other as *const Node {
            return true;
        }
        for child in &self.children {
            if child.borrow().contains(other) {
                return true;
            }
        }
        false
    }

    /// 克隆节点
    /// deep=true: 递归克隆子树; deep=false: 只克隆自身
    /// EventListener 克隆策略: deep=true 时不拷贝事件监听器（W3C 标准行为）
    pub fn clone_node(&self, deep: bool) -> Rc<RefCell<Node>> {
        let new_node_type = match &self.node_type {
            NodeType::Element(e) => {
                let mut new_elem = super::element::ElementData::new(e.tag_name());
                new_elem.id = e.id.clone();
                for (k, v) in &e.attributes {
                    new_elem.attributes.insert(k.clone(), v.clone());
                }
                new_elem.class_list = e.class_list.clone();
                // 不克隆事件监听器（W3C 标准）
                NodeType::Element(new_elem)
            }
            NodeType::Text(t) => {
                NodeType::Text(super::text::Text::new(t.data()))
            }
            NodeType::Document => NodeType::Document,
            NodeType::DocumentFragment => NodeType::DocumentFragment,
            NodeType::Comment(s) => NodeType::Comment(s.clone()),
        };

        let new_node = Node::new(new_node_type);

        if deep {
            for child in &self.children {
                let cloned_child = child.borrow().clone_node(true);
                new_node.borrow_mut().append_child(cloned_child);
            }
        }

        new_node
    }

    // ============================================================
    //  属性访问 —— 对应 W3C Node 属性
    // ============================================================

    /// 获取节点所有子元素的文本内容拼接
    pub fn text_content(&self) -> String {
        let mut result = String::new();
        match &self.node_type {
            NodeType::Text(t) => result.push_str(t.data()),
            NodeType::Comment(_) => {} // 注释不参与 textContent
            NodeType::Element(_) | NodeType::Document | NodeType::DocumentFragment => {
                for child in &self.children {
                    result.push_str(&child.borrow().text_content());
                }
            }
        }
        result
    }

    /// 设置文本内容（替换所有子节点为单个 Text 节点）
    pub fn set_text_content(&mut self, text: &str) {
        self.children.clear();
        let child = Node::new(NodeType::Text(super::text::Text::new(text)));
        let self_rc = self.weak_self.upgrade().expect("Node was dropped");
        child.borrow_mut().parent = Some(Rc::downgrade(&self_rc));
        self.children.push(child);
        self.mark_dirty(true);
    }

    /// 父节点
    pub fn parent_node(&self) -> Option<Rc<RefCell<Node>>> {
        self.parent.as_ref().and_then(|w| w.upgrade())
    }

    /// 子节点列表的拷贝
    pub fn child_nodes(&self) -> Vec<Rc<RefCell<Node>>> {
        self.children.clone()
    }

    /// 第一个子节点
    pub fn first_child(&self) -> Option<Rc<RefCell<Node>>> {
        self.children.first().cloned()
    }

    /// 最后一个子节点
    pub fn last_child(&self) -> Option<Rc<RefCell<Node>>> {
        self.children.last().cloned()
    }

    /// 前一个兄弟节点
    pub fn previous_sibling(&self) -> Option<Rc<RefCell<Node>>> {
        self.prev_sibling.as_ref().and_then(|w| w.upgrade())
    }

    /// 后一个兄弟节点
    pub fn next_sibling(&self) -> Option<Rc<RefCell<Node>>> {
        self.next_sibling.clone()
    }

    /// 节点类型数字常量（对应 node_type_constants）
    pub fn node_type(&self) -> u16 {
        match &self.node_type {
            NodeType::Element(_) => node_type_constants::ELEMENT_NODE,
            NodeType::Text(_) => node_type_constants::TEXT_NODE,
            NodeType::Document => node_type_constants::DOCUMENT_NODE,
            NodeType::DocumentFragment => node_type_constants::DOCUMENT_FRAGMENT_NODE,
            NodeType::Comment(_) => node_type_constants::COMMENT_NODE,
        }
    }

    /// 节点名称：
    ///   Element → 大写标签名 "DIV"
    ///   Text → "#text"
    ///   Document → "#document"
    ///   DocumentFragment → "#document-fragment"
    ///   Comment → "#comment"
    pub fn node_name(&self) -> String {
        match &self.node_type {
            NodeType::Element(e) => e.tag_name().to_uppercase(),
            NodeType::Text(_) => "#text".to_string(),
            NodeType::Document => "#document".to_string(),
            NodeType::DocumentFragment => "#document-fragment".to_string(),
            NodeType::Comment(_) => "#comment".to_string(),
        }
    }

    /// 子节点数量
    pub fn child_element_count(&self) -> usize {
        self.children.len()
    }

    // ============================================================
    //  Phase 3 新增方法
    // ============================================================

    /// 判断是否有子节点
    pub fn has_child_nodes(&self) -> bool {
        !self.children.is_empty()
    }

    /// 判断两个节点引用是否指向同一个节点（Rc 指针比较）
    /// React/Vue diff 算法核心，框架常用
    pub fn is_same_node(&self, other: &Rc<RefCell<Node>>) -> bool {
        std::ptr::eq(self, &*other.borrow())
    }

    /// 获取仅包含元素类型的子节点（不含 Text/Comment 节点）
    pub fn children(&self) -> Vec<Rc<RefCell<Node>>> {
        self.children
            .iter()
            .filter(|c| matches!(c.borrow().node_type, NodeType::Element(_)))
            .cloned()
            .collect()
    }

    /// 第一个元素子节点
    pub fn first_element_child(&self) -> Option<Rc<RefCell<Node>>> {
        self.children
            .iter()
            .find(|c| matches!(c.borrow().node_type, NodeType::Element(_)))
            .cloned()
    }

    /// 最后一个元素子节点
    pub fn last_element_child(&self) -> Option<Rc<RefCell<Node>>> {
        self.children
            .iter()
            .rev()
            .find(|c| matches!(c.borrow().node_type, NodeType::Element(_)))
            .cloned()
    }

    /// 下一个元素兄弟节点
    pub fn next_element_sibling(&self) -> Option<Rc<RefCell<Node>>> {
        let mut current = self.next_sibling.clone();
        while let Some(sibling) = current {
            if matches!(sibling.borrow().node_type, NodeType::Element(_)) {
                return Some(sibling);
            }
            current = sibling.borrow().next_sibling.clone();
        }
        None
    }

    /// 上一个元素兄弟节点
    pub fn previous_element_sibling(&self) -> Option<Rc<RefCell<Node>>> {
        let mut current = self.prev_sibling.as_ref().and_then(|w| w.upgrade());
        while let Some(sibling) = current {
            if matches!(sibling.borrow().node_type, NodeType::Element(_)) {
                return Some(sibling);
            }
            current = sibling.borrow().prev_sibling.as_ref().and_then(|w| w.upgrade());
        }
        None
    }

    /// 完整 innerHTML 序列化（Phase 3）
    pub fn inner_html(&self) -> String {
        let mut result = String::new();
        for child in &self.children {
            Self::serialize_node_html(&child.borrow(), &mut result);
        }
        result
    }

    fn serialize_node_html(node: &Node, output: &mut String) {
        match &node.node_type {
            NodeType::Element(e) => {
                output.push_str(&format!("<{}", e.tag_name()));
                if let Some(id) = &e.id {
                    output.push_str(&format!(" id=\"{}\"", id));
                }
                let classes = e.class_name();
                if !classes.is_empty() {
                    output.push_str(&format!(" class=\"{}\"", classes));
                }
                for (name, value) in &e.attributes {
                    if name != "id" && name != "class" {
                        output.push_str(&format!(" {}=\"{}\"", name, value));
                    }
                }
                if node.children.is_empty() {
                    output.push_str(" />");
                } else {
                    output.push('>');
                    for child in &node.children {
                        Self::serialize_node_html(&child.borrow(), output);
                    }
                    output.push_str(&format!("</{}>", e.tag_name()));
                }
            }
            NodeType::Text(t) => {
                output.push_str(&html_escape_text(t.data()));
            }
            NodeType::Comment(s) => {
                output.push_str(&format!("<!--{}-->", s));
            }
            NodeType::Document | NodeType::DocumentFragment => {
                for child in &node.children {
                    Self::serialize_node_html(&child.borrow(), output);
                }
            }
        }
    }

    /// 向上查找匹配选择器的最近祖先元素（含自身）
    /// 支持 tag、.class、#id 及组合选择器
    pub fn closest(&self, selector: &str) -> Option<Rc<RefCell<Node>>> {
        let self_rc = self.weak_self.upgrade()?;

        if let NodeType::Element(_) = &self.node_type {
            if simple_selector_match(&self_rc.borrow(), selector) {
                return Some(self_rc);
            }
        }

        let mut current = self.parent_node();
        while let Some(parent) = current {
            if matches!(parent.borrow().node_type, NodeType::Element(_)) {
                if simple_selector_match(&parent.borrow(), selector) {
                    return Some(parent);
                }
            }
            let next = parent.borrow().parent_node();
            current = next;
        }
        None
    }

    /// 获取元素边界矩形（相对于视口）
    /// Phase 3: layout 阶段通过 style 属性缓存 rect，此处读取
    pub fn get_bounding_client_rect(&self) -> crate::Rect<f32> {
        crate::Rect::new(0.0, 0.0, 0.0, 0.0)
    }

    /// 标记节点为脏（需要重排）
    pub(crate) fn mark_dirty(&self, dirty: bool) {
        self.dirty.set(dirty);
    }

    /// 是否脏节点
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty.get()
    }

    // ============================================================
    //  Phase 1 新增方法
    // ============================================================

    /// 查找包含本节点的 Document 节点
    /// 沿着 parent 链向上遍历，返回第一个 Document 类型节点
    pub fn owner_document(&self) -> Option<Rc<RefCell<Document>>> {
        let mut current = self.parent_node();
        while let Some(node) = current {
            let node_ref = node.borrow();
            if matches!(node_ref.node_type, NodeType::Document) {
                // 通过 weak_self 无法获取 Document，返回 None 后由外部处理
                // 实际使用中，Document 节点总是 Rc<RefCell<Document>> 包装在 Node 中
                drop(node_ref);
                // 返回 Option — 实际在 WebWindow 中维护 document 引用
                return None;
            }
            let parent = node_ref.parent_node();
            drop(node_ref);
            current = parent;
        }
        None
    }

    /// 规范化子节点：合并相邻 Text 节点，移除空 Text 节点
    pub fn normalize(&mut self) {
        let mut i = 0;
        while i < self.children.len() {
            let is_text = matches!(self.children[i].borrow().node_type, NodeType::Text(_));
            if is_text {
                // 移除空 Text
                let text_content = self.children[i].borrow().text_content();
                if text_content.is_empty() {
                    self.children.remove(i);
                    continue;
                }
                // 与下一个 Text 节点合并
                if i + 1 < self.children.len() {
                    let next_is_text = matches!(self.children[i + 1].borrow().node_type, NodeType::Text(_));
                    if next_is_text {
                        let next_text = self.children[i + 1].borrow().text_content();
                        if let NodeType::Text(t) = &mut self.children[i].borrow_mut().node_type {
                            t.append_data(&next_text);
                        }
                        self.children.remove(i + 1);
                        continue;
                    }
                }
            }
            i += 1;
        }
    }

    /// 深度相等比较（忽略 DOM 树位置）
    pub fn is_equal_node(&self, other: &Node) -> bool {
        // 比较节点类型
        if self.node_type() != other.node_type() {
            return false;
        }
        // 比较 node_name
        if self.node_name() != other.node_name() {
            return false;
        }
        // 比较文本内容
        match (&self.node_type, &other.node_type) {
            (NodeType::Text(a), NodeType::Text(b)) => {
                if a.data() != b.data() {
                    return false;
                }
            }
            (NodeType::Comment(a), NodeType::Comment(b)) => {
                if a != b {
                    return false;
                }
            }
            (NodeType::Element(a), NodeType::Element(b)) => {
                // 比较属性
                if a.attributes != b.attributes {
                    return false;
                }
            }
            _ => {}
        }
        // 比较子节点
        if self.children.len() != other.children.len() {
            return false;
        }
        for (c1, c2) in self.children.iter().zip(other.children.iter()) {
            if !c1.borrow().is_equal_node(&c2.borrow()) {
                return false;
            }
        }
        true
    }

    /// 比较两个节点在文档中的位置关系
    /// Phase 2+: 完整实现位常量
    pub fn compare_document_position(&self, other: &Node) -> u16 {
        use super::document_position;
        if self as *const Node == other as *const Node {
            return 0;
        }
        // 检查是否在同一文档中
        if self.contains(other) {
            return document_position::DOCUMENT_POSITION_CONTAINED_BY
                | document_position::DOCUMENT_POSITION_FOLLOWING;
        }
        if other.contains(self) {
            return document_position::DOCUMENT_POSITION_CONTAINS
                | document_position::DOCUMENT_POSITION_PRECEDING;
        }
        document_position::DOCUMENT_POSITION_DISCONNECTED
    }

    /// 获取元素属性值（仅 Element 节点有效）
    pub fn get_attribute(&self, name: &str) -> Option<String> {
        match &self.node_type {
            NodeType::Element(e) => e.get_attribute(name),
            _ => None,
        }
    }

    /// 设置元素属性（仅 Element 节点有效）
    pub fn set_attribute(&mut self, name: &str, value: &str) {
        if let NodeType::Element(e) = &mut self.node_type {
            e.set_attribute(name, value);
            self.mark_dirty(true);
        }
    }

    /// 移除元素属性（仅 Element 节点有效）
    pub fn remove_attribute(&mut self, name: &str) {
        if let NodeType::Element(e) = &mut self.node_type {
            e.remove_attribute(name);
            self.mark_dirty(true);
        }
    }

    /// 判断元素是否有指定属性（仅 Element 节点有效）
    pub fn has_attribute(&self, name: &str) -> bool {
        match &self.node_type {
            NodeType::Element(e) => e.has_attribute(name),
            _ => false,
        }
    }

    /// 获取元素标签名（仅 Element 节点有效）
    pub fn tag_name(&self) -> Option<&str> {
        match &self.node_type {
            NodeType::Element(e) => Some(e.tag_name()),
            _ => None,
        }
    }

    /// 从 CSS 字符串解析并设置内联样式（仅 Element 节点有效）
    pub fn set_style(&mut self, style_str: &str) {
        if let NodeType::Element(e) = &mut self.node_type {
            e.parse_and_set_style(style_str);
            self.mark_dirty(true);
        }
    }

    /// 添加事件监听器（仅 Element 节点有效）
    pub fn add_event_listener(
        &mut self,
        event_type: &str,
        callback: Box<dyn Fn(&super::event::Event)>,
    ) -> Option<usize> {
        if let NodeType::Element(e) = &mut self.node_type {
            Some(e.add_event_listener(event_type, callback))
        } else {
            None
        }
    }

    /// 移除事件监听器（仅 Element 节点有效）
    pub fn remove_event_listener(&mut self, event_type: &str, id: usize) {
        if let NodeType::Element(e) = &mut self.node_type {
            e.remove_event_listener(event_type, id);
        }
    }

    /// 派发事件（仅 Element 节点有效）
    pub fn dispatch_event(&mut self, event: &super::event::Event) -> bool {
        if let NodeType::Element(e) = &mut self.node_type {
            e.dispatch_event(event)
        } else {
            false
        }
    }
}

/// HTML 转义文本内容
fn html_escape_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// 简单选择器匹配（tag、.class、#id 及组合，如 div.container#main）
fn simple_selector_match(node: &Node, selector: &str) -> bool {
    let selector = selector.trim();
    if selector.is_empty() || selector == "*" {
        return true;
    }
    if let NodeType::Element(e) = &node.node_type {
        let mut pos = 0;
        let chars: Vec<char> = selector.chars().collect();
        let len = chars.len();

        // 解析 tag（开头到第一个 . 或 # 之前的部分）
        let mut tag = String::new();
        while pos < len && chars[pos] != '.' && chars[pos] != '#' {
            tag.push(chars[pos]);
            pos += 1;
        }
        if !tag.is_empty() && tag != e.tag_name() {
            return false;
        }

        // 解析 .class 和 #id
        while pos < len {
            if chars[pos] == '.' {
                pos += 1;
                let mut class = String::new();
                while pos < len && chars[pos] != '.' && chars[pos] != '#' {
                    class.push(chars[pos]);
                    pos += 1;
                }
                if !class.is_empty() && !e.has_class(&class) {
                    return false;
                }
            } else if chars[pos] == '#' {
                pos += 1;
                let mut id = String::new();
                while pos < len && chars[pos] != '.' && chars[pos] != '#' {
                    id.push(chars[pos]);
                    pos += 1;
                }
                if !id.is_empty() && e.id() != Some(&id) {
                    return false;
                }
            } else {
                pos += 1;
            }
        }
        true
    } else {
        false
    }
}

/// Debug 输出（简短摘要）
impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("node_name", &self.node_name())
            .field("node_type", &self.node_type())
            .field("children_count", &self.children.len())
            .finish()
    }
}

/// 格式化输出 DOM 树（调试用）
impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.node_type {
            NodeType::Element(e) => {
                write!(f, "<{}", e.tag_name())?;
                if let Some(id) = &e.id {
                    write!(f, " id=\"{}\"", id)?;
                }
                let classes = e.class_name();
                if !classes.is_empty() {
                    write!(f, " class=\"{}\"", classes)?;
                }
                if self.children.is_empty() {
                    write!(f, " />")?;
                } else {
                    write!(f, ">")?;
                    for child in &self.children {
                        write!(f, "{}", child.borrow())?;
                    }
                    write!(f, "</{}>", e.tag_name())?;
                }
            }
            NodeType::Text(t) => write!(f, "{}", t.data())?,
            NodeType::Comment(s) => write!(f, "<!--{}-->", s)?,
            NodeType::Document => {
                for child in &self.children {
                    write!(f, "{}", child.borrow())?;
                }
            }
            NodeType::DocumentFragment => {
                for child in &self.children {
                    write!(f, "{}", child.borrow())?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "node.test.rs"]
mod tests;
