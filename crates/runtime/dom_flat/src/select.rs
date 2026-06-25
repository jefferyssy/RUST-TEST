//! Select — 选择器 builder（编译期生成代码用）。
//!
//! 链式过程实时累加特异性，`.done()` 时零计算。

use super::selector_match::{
    AttrOp, BasicSelector, Combinator, ComplexSelector, PseudoClass,
    PseudoElement, SelectSegment,
};
use super::specificity::Specificity;

/// 选择器 builder。
pub struct Select {
    segments: Vec<SelectSegment>,
    current: Vec<BasicSelector>,
    next_combinator: Combinator,
    specificity: (u32, u32, u32), // (id, class, tag)
}

impl Select {
    fn new() -> Self {
        Self {
            segments: Vec::new(),
            current: Vec::new(),
            next_combinator: Combinator::Descendant,
            specificity: (0, 0, 0),
        }
    }

    // ── 入口 ──

    pub fn tag(tag: impl Into<String>) -> Self {
        let mut s = Self::new();
        s.current.push(BasicSelector::Tag(tag.into()));
        s.specificity.2 += 1;
        s
    }

    pub fn id(id: impl Into<String>) -> Self {
        let mut s = Self::new();
        s.current.push(BasicSelector::Id(id.into()));
        s.specificity.0 += 1;
        s
    }

    pub fn class(class: impl Into<String>) -> Self {
        let mut s = Self::new();
        s.current.push(BasicSelector::Class(class.into()));
        s.specificity.1 += 1;
        s
    }

    // ── 当前段追加 ──

    pub fn and_tag(mut self, tag: impl Into<String>) -> Self {
        self.current.push(BasicSelector::Tag(tag.into()));
        self.specificity.2 += 1;
        self
    }

    pub fn and_class(mut self, class: impl Into<String>) -> Self {
        self.current.push(BasicSelector::Class(class.into()));
        self.specificity.1 += 1;
        self
    }

    pub fn and_id(mut self, id: impl Into<String>) -> Self {
        self.current.push(BasicSelector::Id(id.into()));
        self.specificity.0 += 1;
        self
    }

    pub fn and_pseudo(mut self, pc: PseudoClass) -> Self {
        self.current.push(BasicSelector::PseudoClass(pc));
        self.specificity.1 += 1;
        self
    }

    pub fn and_pseudo_element(mut self, pe: PseudoElement) -> Self {
        self.current.push(BasicSelector::PseudoElement(pe));
        self
    }

    pub fn and_attr(
        mut self,
        name: impl Into<String>,
        op: AttrOp,
        value: impl Into<String>,
    ) -> Self {
        self.current.push(BasicSelector::Attribute {
            name: name.into(),
            op,
            value: value.into(),
        });
        self.specificity.1 += 1;
        self
    }

    // ── 组合器（提交当前段，设下一段连接方式）──

    pub fn child(mut self) -> Self {
        self.commit_segment();
        self.next_combinator = Combinator::Child;
        self
    }

    pub fn descendant(mut self) -> Self {
        self.commit_segment();
        self.next_combinator = Combinator::Descendant;
        self
    }

    pub fn adjacent(mut self) -> Self {
        self.commit_segment();
        self.next_combinator = Combinator::Adjacent;
        self
    }

    pub fn sibling(mut self) -> Self {
        self.commit_segment();
        self.next_combinator = Combinator::Sibling;
        self
    }

    // ── 产出 ──

    /// 完成构建，产出 ComplexSelector。
    pub fn done(mut self) -> ComplexSelector {
        self.commit_segment();
        ComplexSelector {
            segments: self.segments,
            specificity: Specificity(
                self.specificity.0,
                self.specificity.1,
                self.specificity.2,
            ),
        }
    }

    // ── 内部 ──

    fn commit_segment(&mut self) {
        if self.current.is_empty() {
            return;
        }
        let combinator = if self.segments.is_empty() {
            Combinator::Descendant
        } else {
            self.next_combinator
        };
        self.segments.push(SelectSegment {
            combinator,
            parts: std::mem::take(&mut self.current),
        });
    }
}
