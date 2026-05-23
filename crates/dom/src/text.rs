//! Text 文本节点

use std::cell::RefCell;
use std::rc::Rc;

use super::node::{Node, NodeType};

/// 文本节点
pub struct Text {
    data: String,
}

impl Text {
    /// 创建文本节点
    pub fn new(data: &str) -> Self {
        Self { data: data.to_string() }
    }

    /// 获取文本内容
    pub fn data(&self) -> &str {
        &self.data
    }

    /// 设置文本内容
    pub fn set_data(&mut self, data: &str) {
        self.data = data.to_string();
    }

    /// 文本长度
    pub fn length(&self) -> usize {
        self.data.len()
    }

    // ============================================================
    //  Phase 1 新增 —— 文本操作方法（W3C Text 接口）
    // ============================================================

    /// 在 offset 位置分割文本节点
    /// 当前节点保留 [0..offset)，返回包含 [offset..] 的新节点
    /// 新节点自动插入到当前节点之后
    pub fn split_text(
        &mut self,
        offset: usize,
        parent: &Rc<RefCell<Node>>,
        self_rc: &Rc<RefCell<Node>>,
    ) -> Rc<RefCell<Node>> {
        let offset = offset.min(self.data.len());
        let right_data = self.data[offset..].to_string();
        self.data.truncate(offset);

        let new_node = Node::new(NodeType::Text(Text::new(&right_data)));
        let next = self_rc.borrow().next_sibling();

        if let Some(next_rc) = next {
            parent.borrow_mut().insert_before(new_node.clone(), Some(&next_rc.borrow()));
        } else {
            parent.borrow_mut().append_child(new_node.clone());
        }
        new_node
    }

    /// 追加文本到末尾
    pub fn append_data(&mut self, data: &str) {
        self.data.push_str(data);
    }

    /// 删除 offset 开始、count 长度的文本
    pub fn delete_data(&mut self, offset: usize, count: usize) {
        let offset = offset.min(self.data.len());
        let end = (offset + count).min(self.data.len());
        self.data.replace_range(offset..end, "");
    }

    /// 在 offset 位置插入文本
    pub fn insert_data(&mut self, offset: usize, data: &str) {
        let offset = offset.min(self.data.len());
        self.data.insert_str(offset, data);
    }

    /// 替换 offset 开始、count 长度的文本
    pub fn replace_data(&mut self, offset: usize, count: usize, data: &str) {
        let offset = offset.min(self.data.len());
        let end = (offset + count).min(self.data.len());
        self.data.replace_range(offset..end, data);
    }

    /// 提取 offset 开始、count 长度的子串
    pub fn substring_data(&self, offset: usize, count: usize) -> String {
        let offset = offset.min(self.data.len());
        let end = (offset + count).min(self.data.len());
        self.data[offset..end].to_string()
    }
}

#[cfg(test)]
#[path = "text.test.rs"]
mod tests;
