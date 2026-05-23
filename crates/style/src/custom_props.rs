//! CSS 自定义属性 (Custom Properties / CSS Variables) —— Phase 2
//!
//! 对应 W3C CSS Custom Properties for Cascading Variables Module Level 1。
//! 支持 --custom-name: value 声明和 var(--custom-name) 引用。

use std::collections::HashMap;

use crate::values::CSSValue;

/// 自定义属性注册表
///
/// 作用域链：元素局部 > 父元素继承 > :root 全局
#[derive(Debug, Clone, Default)]
pub struct CustomPropertyRegistry {
    /// 全局（:root）自定义属性
    global: HashMap<String, CSSValue>,
}

impl CustomPropertyRegistry {
    pub fn new() -> Self {
        Self {
            global: HashMap::new(),
        }
    }

    /// 在 :root 级别设置自定义属性
    pub fn set_global(&mut self, name: &str, value: CSSValue) {
        self.global.insert(name.to_string(), value);
    }

    /// 获取全局自定义属性
    pub fn get_global(&self, name: &str) -> Option<&CSSValue> {
        self.global.get(name)
    }

    /// 移除全局自定义属性
    pub fn remove_global(&mut self, name: &str) {
        self.global.remove(name);
    }

    /// 解析属性值中的 var() 引用
    ///
    /// var(--name) → 查找自定义属性值
    /// var(--name, fallback) → 使用后备值
    pub fn resolve_var(
        &self,
        value: &str,
        local_props: &HashMap<String, CSSValue>,
        inherited_props: &HashMap<String, CSSValue>,
    ) -> Option<CSSValue> {
        let value = value.trim();
        // 检测 var( 引用
        if let Some(inner) = value
            .strip_prefix("var(")
            .and_then(|s| s.strip_suffix(')'))
        {
            let inner = inner.trim();
            // 解析: --name 或 --name, fallback
            if let Some(comma_idx) = inner.find(',') {
                let var_name = inner[..comma_idx].trim();
                let fallback = inner[comma_idx + 1..].trim();

                self.lookup(var_name, local_props, inherited_props)
                    .or_else(|| {
                        // 后备值：按普通 CSS 值解析
                        Some(CSSValue::Keyword(fallback.to_string()))
                    })
            } else {
                self.lookup(inner, local_props, inherited_props)
            }
        } else {
            None
        }
    }

    /// 按作用域链查找自定义属性
    fn lookup(
        &self,
        name: &str,
        local_props: &HashMap<String, CSSValue>,
        inherited_props: &HashMap<String, CSSValue>,
    ) -> Option<CSSValue> {
        // 1. 局部范围
        if let Some(val) = local_props.get(name) {
            return Some(val.clone());
        }
        // 2. 继承范围
        if let Some(val) = inherited_props.get(name) {
            return Some(val.clone());
        }
        // 3. 全局范围
        self.global.get(name).cloned()
    }

    /// 收集所有自定义属性声明（以 -- 开头的属性）
    pub fn collect_custom_properties(
        declarations: &[(String, String)],
    ) -> HashMap<String, CSSValue> {
        let mut props = HashMap::new();
        for (name, value) in declarations {
            if name.starts_with("--") {
                props.insert(
                    name.clone(),
                    CSSValue::Keyword(value.clone()),
                );
            }
        }
        props
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_set_get() {
        let mut reg = CustomPropertyRegistry::new();
        reg.set_global("--main-color", CSSValue::Keyword("red".to_string()));
        assert!(reg.get_global("--main-color").is_some());
    }

    #[test]
    fn test_collect_custom_props() {
        let decls = vec![
            ("--main-color".to_string(), "#ff0000".to_string()),
            ("color".to_string(), "red".to_string()),
            ("--spacing".to_string(), "16px".to_string()),
        ];
        let props = CustomPropertyRegistry::collect_custom_properties(&decls);
        assert_eq!(props.len(), 2);
        assert!(props.contains_key("--main-color"));
        assert!(props.contains_key("--spacing"));
    }
}
