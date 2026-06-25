//! VNode — 虚拟 DOM 节点（html! 宏 / 编译器 的纯语法脱糖产物）。
//!
//! VNode 是"用户声明了什么 UI"，DomRegistry 是"运行时的真实 DOM 状态"。
//! 后续 reconciliation diff 对比新旧 VNode → 算出最小变更 → 应用到 DomRegistry。

/// 虚拟 DOM 节点。
#[derive(Debug, Clone)]
pub enum VNode {
    /// 元素节点
    Element(VElementVNode),
    /// 文本节点
    Text(String),
}

/// 虚拟元素节点。对标浏览器 HTMLElement。
#[derive(Debug, Clone)]
pub struct VElementVNode {
    /// 标签名（小写），例如 "div", "button"
    pub tagName: String,
    /// id 属性
    pub id: Option<String>,
    /// class 列表
    pub classList: Vec<String>,
    /// 内联样式原始字符串
    pub style: String,
    /// 其他属性键值对
    pub attrs: Vec<(String, String)>,
    /// 子 VNode 列表
    pub childNodes: Vec<VNode>,
    /// diff 复用 key（一期暂未使用，预留给 reconciliation）
    pub diffKey: Option<String>,
}

impl Default for VElementVNode {
    fn default() -> Self {
        Self {
            tagName: String::new(),
            id: None,
            classList: Vec::new(),
            style: String::new(),
            attrs: Vec::new(),
            childNodes: Vec::new(),
            diffKey: None,
        }
    }
}

impl VElementVNode {
    /// 快速构造一个纯标签元素（无属性、无子节点）。
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            tagName: tag.into(),
            ..Default::default()
        }
    }

    /// 追加子节点。
    pub fn with_child(mut self, child:impl Into<VNode>) -> Self {
        self.childNodes.push(child.into());
        self
    }

    /// 批量追加子节点。
    pub fn with_children(mut self, children: impl IntoIterator<Item = impl Into<VNode>>) -> Self {
        self.childNodes.extend(children.into_iter().map(|c| c.into()));
        self
    }

    /// 设置 id。
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// 设置 class。
    pub fn with_class(mut self, class: impl Into<String>) -> Self {
        self.classList.push(class.into());
        self
    }

    /// 批量设置 class（自动 into String）。
    pub fn with_classes(mut self, classes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.classList = classes.into_iter().map(|c| c.into()).collect();
        self
    }

    /// 设置内联 style。
    pub fn with_style(mut self, style: impl Into<String>) -> Self {
        self.style = style.into();
        self
    }

    /// 添加属性。
    pub fn with_attr(mut self, key: impl Into<String>, val: impl Into<String>) -> Self {
        self.attrs.push((key.into(), val.into()));
        self
    }
}

impl VNode {
    /// 快速构造文本节点。
    pub fn text(s: impl Into<String>) -> Self {
        VNode::Text(s.into())
    }

    /// 快速构造元素节点。
    pub fn element(tag: impl Into<String>) -> VElementVNode {
        VElementVNode::new(tag)
    }
}

impl From<VElementVNode> for VNode {
    fn from(elem: VElementVNode) -> Self {
        VNode::Element(elem)
    }
}
