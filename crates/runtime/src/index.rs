//! index — CSS 规则双层索引。

use std::collections::{HashMap, HashSet};

use crate::{BasicSelector, CssRule};

/// 规则索引 — 按最右段最佳 key 分组，存 rules 中的下标。
#[derive(Debug, Clone, Default)]
pub struct RuleIndex {
    pub id: HashMap<String, Vec<usize>>,
    pub class: HashMap<String, Vec<usize>>,
    pub tag: HashMap<String, Vec<usize>>,
    pub attr: HashMap<String, Vec<usize>>,
    pub universal: Vec<usize>,
}

impl RuleIndex {
    /// 从规则列表构建索引。
    pub fn build(rules: &[CssRule]) -> Self {
        let mut id: HashMap<String, Vec<usize>> = HashMap::new();
        let mut class: HashMap<String, Vec<usize>> = HashMap::new();
        let mut tag: HashMap<String, Vec<usize>> = HashMap::new();
        let mut attr: HashMap<String, Vec<usize>> = HashMap::new();
        let mut universal = Vec::new();

        for (i, rule) in rules.iter().enumerate() {
            let last = match rule.selector.last_segment() {
                Some(seg) => seg,
                None => {
                    universal.push(i);
                    continue;
                }
            };

            // 取最佳 key: id > class > attr > tag > universal
            if let Some(key) = last.parts.iter().find_map(|p| match p {
                BasicSelector::Id(id_) => Some(id_.clone()),
                _ => None,
            }) {
                id.entry(key).or_default().push(i);
            } else if let Some(key) = last.parts.iter().find_map(|p| match p {
                BasicSelector::Class(c) => Some(c.clone()),
                _ => None,
            }) {
                class.entry(key).or_default().push(i);
            } else if let Some(key) = last.parts.iter().find_map(|p| match p {
                BasicSelector::Attribute { name, .. } => Some(name.clone()),
                _ => None,
            }) {
                attr.entry(key).or_default().push(i);
            } else if let Some(key) = last.parts.iter().find_map(|p| match p {
                BasicSelector::Tag(t) => Some(t.clone()),
                _ => None,
            }) {
                tag.entry(key).or_default().push(i);
            } else {
                universal.push(i);
            }
        }

        Self { id, class, tag, attr, universal }
    }

    /// 根据元素属性查找候选规则下标。
    pub fn find_candidates(
        &self,
        tag_name: Option<&str>,
        class_list: &[String],
        element_id: Option<&str>,
        attrs: &[(String, String)],
    ) -> Vec<usize> {
        let mut seen = HashSet::new();
        let mut candidates = Vec::new();

        let mut add = |indices: &[usize]| {
            for &i in indices {
                if seen.insert(i) {
                    candidates.push(i);
                }
            }
        };

        // tag
        if let Some(t) = tag_name {
            if let Some(list) = self.tag.get(&t.to_lowercase()) {
                add(list);
            }
        }
        // class
        for c in class_list {
            if let Some(list) = self.class.get(c) {
                add(list);
            }
        }
        // id
        if let Some(i) = element_id {
            if let Some(list) = self.id.get(i) {
                add(list);
            }
        }
        // attr
        for (name, _) in attrs {
            if let Some(list) = self.attr.get(name) {
                add(list);
            }
        }
        // universal
        add(&self.universal);

        candidates
    }
}
