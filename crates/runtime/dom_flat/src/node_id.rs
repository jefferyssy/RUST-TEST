//! NodeId — 扁平化 DOM 节点的全局唯一标识符。
//!
//! 对标浏览器内部的对象引用，但使用 u64 而非指针以支持序列化和跨线程传递。

/// 全局节点 ID，u64 单调递增分配。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u64);

impl NodeId {
    /// 从 u64 构造 NodeId。
    #[inline]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// 获取内部 u64 值。
    #[inline]
    pub fn value(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}
