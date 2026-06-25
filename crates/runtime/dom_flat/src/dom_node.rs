//! DomNode — 扁平化 DOM 节点（全小驼峰，完全对标浏览器 Element 原生属性）。

use super::{ComputedStyle, NodeId, Rect};

/// 扁平化 DOM 节点。
///
/// 不再使用树形 `Rc<RefCell<Node>>`，而是通过 NodeId 在 DomRegistry 中按 ID 查找。
/// 所有字段使用小驼峰，与浏览器原生 JS `Element` 属性命名完全一致。
#[derive(Debug, Clone)]
pub struct DomNode {
    /// 自身 ID
    pub nodeId: NodeId,
    /// 父节点 ID（对标 `element.parentNode`）
    pub parentNode: Option<NodeId>,
    /// 子节点 ID 列表（对标 `element.childNodes`）
    pub childNodes: Vec<NodeId>,

    // ── HTML 层属性 ──
    /// 标签名小写（对标 `element.tagName`），文本节点为 None
    pub tagName: Option<String>,
    /// id 属性（对标 `element.id`）
    pub id: Option<String>,
    /// class 列表（对标 `element.classList`）
    pub classList: Vec<String>,
    /// 内联样式原始字符串（对标 `element.style`）
    pub style: String,
    /// 其他属性键值对（对标 `element.getAttribute`）
    pub attrs: Vec<(String, String)>,

    // ── 计算样式 ──
    /// 运行时计算后的样式
    pub computedStyle: ComputedStyle,
    /// 样式脏标记：属性/class/id 变更后置 true
    pub styleDirty: bool,

    // ── 布局 ──
    /// 当前布局矩形
    pub layoutRect: Rect,
    /// 上一帧布局矩形，用于脏矩形计算
    pub prevBounds: Rect,
    /// 布局脏标记
    pub layoutDirty: bool,

    // ── 绘制 ──
    /// 绘制脏标记
    pub paintDirty: bool,
}

impl DomNode {
    /// 创建元素节点。
    pub fn new_element(nodeId: NodeId, tagName: String) -> Self {
        Self {
            nodeId,
            parentNode: None,
            childNodes: Vec::new(),
            tagName: Some(tagName),
            id: None,
            classList: Vec::new(),
            style: String::new(),
            attrs: Vec::new(),
            computedStyle: ComputedStyle::default(),
            styleDirty: true,
            layoutRect: Rect::default(),
            prevBounds: Rect::default(),
            layoutDirty: true,
            paintDirty: true,
        }
    }

    /// 创建文本节点。
    pub fn new_text(nodeId: NodeId) -> Self {
        Self {
            nodeId,
            parentNode: None,
            childNodes: Vec::new(),
            tagName: None,
            id: None,
            classList: Vec::new(),
            style: String::new(),
            attrs: Vec::new(),
            computedStyle: ComputedStyle::default(),
            styleDirty: false,
            layoutRect: Rect::default(),
            prevBounds: Rect::default(),
            layoutDirty: false,
            paintDirty: false,
        }
    }

    /// 是否元素节点。
    pub fn is_element(&self) -> bool {
        self.tagName.is_some()
    }

    /// 是否文本节点。
    pub fn is_text(&self) -> bool {
        self.tagName.is_none()
    }

    /// 获取文本内容（仅文本节点有效）。
    pub fn text_content(&self) -> Option<&str> {
        // 文本节点的内容存在 attrs[0] 中，由 createTextNode 设置
        self.attrs.first().map(|(_, v)| v.as_str())
    }
}
