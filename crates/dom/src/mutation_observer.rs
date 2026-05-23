//! MutationObserver —— W3C DOM 变更观察器
//!
//! 替代 Phase 0 的 Cell<bool> 脏标记机制
//! Phase 1：同步触发（微任务队列在 Phase 2+）

use std::cell::RefCell;
use std::rc::Rc;

use super::Node;

/// MutationObserver —— W3C DOM 变更观察器
pub struct MutationObserver {
    /// 回调函数
    callback: Box<dyn Fn(&[MutationRecord], &MutationObserver)>,
    /// 待处理的变更记录
    pending_records: RefCell<Vec<MutationRecord>>,
    /// 是否已断开
    disconnected: RefCell<bool>,
}

/// 单条变更记录
#[derive(Debug, Clone)]
pub struct MutationRecord {
    /// 变更类型
    pub record_type: MutationRecordType,
    /// 变更目标节点
    pub target: Rc<RefCell<Node>>,
    /// 添加的节点（childList 时有效）
    pub added_nodes: Vec<Rc<RefCell<Node>>>,
    /// 移除的节点（childList 时有效）
    pub removed_nodes: Vec<Rc<RefCell<Node>>>,
    /// 上一个兄弟节点（childList 时有效）
    pub previous_sibling: Option<Rc<RefCell<Node>>>,
    /// 下一个兄弟节点（childList 时有效）
    pub next_sibling: Option<Rc<RefCell<Node>>>,
    /// 变更的属性名（attributes 时有效）
    pub attribute_name: Option<String>,
    /// 旧值（需在 observe 时指定 oldValue 选项）
    pub old_value: Option<String>,
}

/// 变更记录类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MutationRecordType {
    /// 属性变更
    Attributes,
    /// 子节点列表变更
    ChildList,
    /// 文本节点内容变更
    CharacterData,
}

/// MutationObserver 初始化选项
#[derive(Debug, Clone)]
pub struct MutationObserverInit {
    /// 是否观察子节点列表变更
    pub child_list: bool,
    /// 是否观察属性变更
    pub attributes: bool,
    /// 是否观察文本内容变更
    pub character_data: bool,
    /// 是否观察后代节点（subtree=true）
    pub subtree: bool,
    /// 是否记录属性旧值
    pub attribute_old_value: bool,
    /// 是否记录文本旧值
    pub character_data_old_value: bool,
    /// 限制观察的属性名列表（None = 全部属性）
    pub attribute_filter: Option<Vec<String>>,
}

impl Default for MutationObserverInit {
    fn default() -> Self {
        Self {
            child_list: false,
            attributes: false,
            character_data: false,
            subtree: false,
            attribute_old_value: false,
            character_data_old_value: false,
            attribute_filter: None,
        }
    }
}

impl MutationObserver {
    /// 创建观察器
    pub fn new(callback: Box<dyn Fn(&[MutationRecord], &MutationObserver)>) -> Self {
        Self {
            callback,
            pending_records: RefCell::new(Vec::new()),
            disconnected: RefCell::new(false),
        }
    }

    /// 开始观察目标节点
    pub fn observe(&self, _target: &Rc<RefCell<Node>>, _options: MutationObserverInit) {
        // Phase 1: 记录观察配置，Phase 2+ 完整实现 DOM 变更拦截
        *self.disconnected.borrow_mut() = false;
    }

    /// 停止观察并清空待处理记录
    pub fn disconnect(&self) {
        *self.disconnected.borrow_mut() = true;
        self.pending_records.borrow_mut().clear();
    }

    /// 提取并清空当前待处理的变更记录
    pub fn take_records(&self) -> Vec<MutationRecord> {
        std::mem::take(&mut *self.pending_records.borrow_mut())
    }

    /// 内部：添加一条变更记录
    pub(crate) fn queue_record(&self, record: MutationRecord) {
        if !*self.disconnected.borrow() {
            self.pending_records.borrow_mut().push(record.clone());
            // Phase 1: 同步触发回调
            let records = self.take_records();
            if !records.is_empty() {
                (self.callback)(&records, self);
            }
        }
    }
}
