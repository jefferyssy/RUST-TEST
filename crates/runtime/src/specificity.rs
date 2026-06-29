//! specificity — CSS 选择器特异性计算。

use super::selector_match::BasicSelector;

/// 特异性三元组 `(id数, class数, tag数)`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Specificity(pub u32, pub u32, pub u32);

impl Specificity {
    /// 从选择器片段列表计算特异性。
    pub fn from_parts(parts: &[BasicSelector]) -> Self {
        let mut id_count = 0u32;
        let mut class_count = 0u32;
        let mut tag_count = 0u32;

        for part in parts {
            match part {
                BasicSelector::Id(_) => id_count += 1,
                BasicSelector::Class(_)
                | BasicSelector::Attribute { .. }
                | BasicSelector::PseudoClass(_) => class_count += 1,
                BasicSelector::Tag(_) => tag_count += 1,
                BasicSelector::Universal | BasicSelector::PseudoElement(_) => {}
            }
        }

        Self(id_count, class_count, tag_count)
    }

    /// 内联样式使用最高特异性。
    pub fn inline_style() -> Self {
        Self(u32::MAX, 0, 0)
    }
}
