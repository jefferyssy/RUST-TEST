//! Document 文档对象
//!
//! 对应 W3C document 接口

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use super::node::{Node, NodeType};
use super::element::ElementData;
use super::text::Text;

/// Document 对象
pub struct Document {
    pub(crate) document_element: Rc<RefCell<Node>>,
    pub(crate) body: Rc<RefCell<Node>>,
    /// Phase 1: 按 ID 索引元素的 HashMap（O(1) 查找）
    pub(crate) element_id_map: RefCell<HashMap<String, Weak<RefCell<Node>>>>,
    /// Phase 1: <title> 文本内容缓存
    pub(crate) title: RefCell<String>,
    /// Phase 3: Cookie 存储
    pub(crate) cookie: RefCell<String>,
}

impl Document {
    /// 创建新文档（含默认 html > head + body 结构）
    pub fn new() -> Rc<RefCell<Self>> {
        let doc = Rc::new(RefCell::new(Self {
            document_element: Node::new(NodeType::Element(
                ElementData::new("html"),
            )),
            body: Node::new(NodeType::Element(
                ElementData::new("body"),
            )),
            element_id_map: RefCell::new(HashMap::new()),
            title: RefCell::new(String::new()),
            cookie: RefCell::new(String::new()),
        }));

        // 构建 html > body 结构
        let html = doc.borrow().document_element.clone();
        let body = doc.borrow().body.clone();
        html.borrow_mut().append_child(body);

        doc
    }

    /// 创建元素节点：document.createElement("div")
    pub fn create_element(&self, tag_name: &str) -> Rc<RefCell<Node>> {
        let elem = Node::new(NodeType::Element(ElementData::new(tag_name)));
        // 如果有 id 属性，自动注册到索引
        if let Some(id) = elem.borrow().get_attribute("id") {
            self.register_element_by_id(&id, &elem);
        }
        elem
    }

    /// 创建文本节点：document.createTextNode("hello")
    pub fn create_text_node(&self, data: &str) -> Rc<RefCell<Node>> {
        Node::new(NodeType::Text(Text::new(data)))
    }

    /// 创建文档片段节点（用于批量插入优化）
    pub fn create_document_fragment(&self) -> Rc<RefCell<Node>> {
        Node::new(NodeType::DocumentFragment)
    }

    /// 创建注释节点
    pub fn create_comment(&self, data: &str) -> Rc<RefCell<Node>> {
        Node::new(NodeType::Comment(data.to_string()))
    }

    /// 获取文档元素 (<html>)
    pub fn document_element(&self) -> Rc<RefCell<Node>> {
        self.document_element.clone()
    }

    /// 获取 body 元素
    pub fn body(&self) -> Rc<RefCell<Node>> {
        self.body.clone()
    }

    // ============================================================
    //  Phase 1 新增 —— 查询
    // ============================================================

    /// 通过 ID 查找元素（O(1) HashMap 索引）
    pub fn get_element_by_id(&self, id: &str) -> Option<Rc<RefCell<Node>>> {
        self.element_id_map
            .borrow()
            .get(id)
            .and_then(|weak| weak.upgrade())
    }

    /// 按标签名查找所有元素（当前为快照遍历，Phase 2+ 实时集合）
    pub fn get_elements_by_tag_name(&self, tag: &str) -> Vec<Rc<RefCell<Node>>> {
        let mut result = Vec::new();
        self.collect_by_tag(&self.document_element, &tag.to_lowercase(), &mut result);
        result
    }

    fn collect_by_tag(
        &self,
        node: &Rc<RefCell<Node>>,
        tag: &str,
        result: &mut Vec<Rc<RefCell<Node>>>,
    ) {
        let n = node.borrow();
        if let Some(t) = n.tag_name() {
            if t == tag || tag == "*" {
                result.push(node.clone());
            }
        }
        for child in &n.children {
            self.collect_by_tag(child, tag, result);
        }
    }

    /// 按类名查找所有元素
    pub fn get_elements_by_class_name(&self, class: &str) -> Vec<Rc<RefCell<Node>>> {
        let mut result = Vec::new();
        self.collect_by_class(&self.document_element, class, &mut result);
        result
    }

    fn collect_by_class(
        &self,
        node: &Rc<RefCell<Node>>,
        class: &str,
        result: &mut Vec<Rc<RefCell<Node>>>,
    ) {
        let n = node.borrow();
        if let NodeType::Element(e) = &n.node_type {
            if e.class_list().contains(&class.to_string()) {
                result.push(node.clone());
            }
        }
        for child in &n.children {
            self.collect_by_class(child, class, result);
        }
    }

    // ============================================================
    //  Phase 1 新增 —— 节点创建 (见上方)
    // ============================================================

    // ============================================================
    //  Phase 1 新增 —— 文档属性
    // ============================================================

    /// 读取 <title> 文本内容
    pub fn title(&self) -> String {
        self.title.borrow().clone()
    }

    /// 设置 <title> 文本内容
    pub fn set_title(&self, title: &str) {
        *self.title.borrow_mut() = title.to_string();
    }

    // ============================================================
    //  元素 ID 索引管理
    // ============================================================

    /// 注册元素到 ID 索引
    pub fn register_element_by_id(&self, id: &str, node: &Rc<RefCell<Node>>) {
        self.element_id_map
            .borrow_mut()
            .insert(id.to_string(), Rc::downgrade(node));
    }

    /// 从 ID 索引中移除元素
    pub fn unregister_element_id(&self, id: &str) {
        self.element_id_map.borrow_mut().remove(id);
    }

    // ============================================================
    //  Phase 2: 命名空间 + 节点导入/采用
    // ============================================================

    /// 使用命名空间创建元素
    pub fn create_element_ns(&self, _namespace: &str, tag: &str) -> Rc<RefCell<Node>> {
        // Phase 2: 基础实现忽略命名空间（SVG/MathML 命名空间 Phase 3+）
        self.create_element(tag)
    }

    /// 从另一个文档导入节点（深拷贝，分配新所有权）
    pub fn import_node(&self, node: &Node, deep: bool) -> Rc<RefCell<Node>> {
        // Phase 2: clone_node + 转移所有权到当前文档
        node.clone_node(deep)
    }

    /// 采用来自另一个文档的节点（转移所有权，不解绑原位置）
    pub fn adopt_node(&self, node: Rc<RefCell<Node>>) -> Rc<RefCell<Node>> {
        // Phase 2: 设置 owner_document 为当前文档
        let element_id = {
            let node_ref = node.borrow();
            if let NodeType::Element(e) = &node_ref.node_type {
                e.id().cloned()
            } else {
                None
            }
        };
        if let Some(id) = element_id {
            self.register_element_by_id(&id, &node);
        }
        node
    }

    // ============================================================
    //  Phase 3: Cookie
    // ============================================================

    /// 读取 Cookie
    pub fn cookie(&self) -> String {
        self.cookie.borrow().clone()
    }

    /// 设置 Cookie
    pub fn set_cookie(&self, cookie_str: &str) {
        *self.cookie.borrow_mut() = cookie_str.to_string();
    }
}

#[cfg(test)]
#[path = "document.test.rs"]
mod tests;
