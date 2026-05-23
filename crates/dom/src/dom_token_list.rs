//! DOMTokenList —— classList 操作

/// W3C DOMTokenList —— classList 操作接口
#[derive(Debug, Clone)]
pub struct DOMTokenList {
    tokens: Vec<String>,
}

impl DOMTokenList {
    /// 从空格分隔的字符串创建
    pub fn from_string(class_str: &str) -> Self {
        Self {
            tokens: class_str.split_whitespace().map(String::from).collect(),
        }
    }

    /// 是否包含指定类名
    pub fn contains(&self, token: &str) -> bool {
        self.tokens.contains(&token.to_string())
    }

    /// 添加类名（不重复添加）
    pub fn add(&mut self, token: &str) {
        if !self.contains(token) {
            self.tokens.push(token.to_string());
        }
    }

    /// 移除类名
    pub fn remove(&mut self, token: &str) {
        self.tokens.retain(|t| t != token);
    }

    /// 切换类名：存在则移除，不存在则添加，返回切换后状态
    pub fn toggle(&mut self, token: &str) -> bool {
        if self.contains(token) {
            self.remove(token);
            false
        } else {
            self.add(token);
            true
        }
    }

    /// 转为空格分隔的字符串
    pub fn to_string(&self) -> String {
        self.tokens.join(" ")
    }

    // ============================================================
    //  Phase 1 新增
    // ============================================================

    /// 类名数量
    pub fn length(&self) -> usize {
        self.tokens.len()
    }

    /// 通过索引获取类名（越界返回 None）
    pub fn item(&self, index: usize) -> Option<&str> {
        self.tokens.get(index).map(|s| s.as_str())
    }

    /// 替换类名：old_token 存在则替换为 new_token，返回是否成功替换
    pub fn replace(&mut self, old_token: &str, new_token: &str) -> bool {
        if let Some(pos) = self.tokens.iter().position(|t| t == old_token) {
            if !self.tokens.contains(&new_token.to_string()) {
                self.tokens[pos] = new_token.to_string();
                return true;
            }
        }
        false
    }

    /// 检查 token 是否为有效的 CSS 类名
    pub fn supports(&self, token: &str) -> bool {
        !token.is_empty()
            && !token.contains(char::is_whitespace)
            && token.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    }
}

#[cfg(test)]
#[path = "dom_token_list.test.rs"]
mod tests;
