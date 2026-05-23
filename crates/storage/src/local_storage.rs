//! Web Storage API —— Phase 2
//!
//! 对应 W3C Web Storage (Second Edition)。
//! 提供 localStorage 和 sessionStorage 的 Rust 实现。

use std::collections::HashMap;

/// 存储后端 trait
///
/// 允许注入不同的持久化策略：
/// - LocalStorage: 文件系统持久化（应用关闭后保留）
/// - SessionStorage: 内存存储（会话结束后清除）
pub trait StorageBackend {
    /// 获取键值对数量
    fn length(&self) -> usize;

    /// 按索引获取键名
    fn key(&self, index: usize) -> Option<String>;

    /// 获取键对应的值
    fn get_item(&self, key: &str) -> Option<String>;

    /// 设置键值对
    fn set_item(&mut self, key: &str, value: &str);

    /// 移除键值对
    fn remove_item(&mut self, key: &str);

    /// 清空所有数据
    fn clear(&mut self);

    /// 获取所有键值对（用于持久化）
    fn entries(&self) -> Vec<(String, String)>;

    /// 批量恢复（用于从持久化存储加载）
    fn load_entries(&mut self, entries: &[(String, String)]);
}

/// 内存存储（HashMap 后端）
#[derive(Debug, Clone, Default)]
pub struct MemoryStorage {
    data: HashMap<String, String>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}

impl StorageBackend for MemoryStorage {
    fn length(&self) -> usize {
        self.data.len()
    }

    fn key(&self, index: usize) -> Option<String> {
        self.data.keys().nth(index).cloned()
    }

    fn get_item(&self, key: &str) -> Option<String> {
        self.data.get(key).cloned()
    }

    fn set_item(&mut self, key: &str, value: &str) {
        self.data.insert(key.to_string(), value.to_string());
    }

    fn remove_item(&mut self, key: &str) {
        self.data.remove(key);
    }

    fn clear(&mut self) {
        self.data.clear();
    }

    fn entries(&self) -> Vec<(String, String)> {
        self.data.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    fn load_entries(&mut self, entries: &[(String, String)]) {
        for (k, v) in entries {
            self.data.insert(k.clone(), v.clone());
        }
    }
}

/// localStorage — 持久化键值存储
///
/// Phase 2: 使用 MemoryStorage，应用退出时写入文件。
/// Phase 3+: 索引写入、事务性更新。
#[derive(Debug)]
pub struct LocalStorage {
    backend: MemoryStorage,
    /// 存储文件路径（用于持久化）
    file_path: Option<String>,
}

impl LocalStorage {
    /// 创建 localStorage 实例
    pub fn new() -> Self {
        Self {
            backend: MemoryStorage::new(),
            file_path: None,
        }
    }

    /// 绑定持久化文件路径
    pub fn with_file(mut self, path: &str) -> Self {
        self.file_path = Some(path.to_string());
        // 尝试从文件加载
        if let Ok(data) = std::fs::read_to_string(path) {
            // Phase 2: 简单 JSON-like 行解析
            for line in data.lines() {
                if let Some((key, value)) = line.split_once('=') {
                    self.backend.set_item(key, value);
                }
            }
        }
        self
    }

    /// 获取键值对数量
    pub fn length(&self) -> usize {
        self.backend.length()
    }

    /// 获取键对应的值
    pub fn get_item(&self, key: &str) -> Option<String> {
        self.backend.get_item(key)
    }

    /// 设置键值对
    pub fn set_item(&mut self, key: &str, value: &str) {
        self.backend.set_item(key, value);
    }

    /// 移除键值对
    pub fn remove_item(&mut self, key: &str) {
        self.backend.remove_item(key);
    }

    /// 清空所有数据
    pub fn clear(&mut self) {
        self.backend.clear();
        // 清除持久化文件
        if let Some(ref path) = self.file_path {
            let _ = std::fs::remove_file(path);
        }
    }

    /// 持久化到文件
    pub fn persist(&self) -> std::io::Result<()> {
        if let Some(ref path) = self.file_path {
            let mut content = String::new();
            for (k, v) in self.backend.entries() {
                content.push_str(&format!("{}={}\n", k, v));
            }
            std::fs::write(path, content)?;
        }
        Ok(())
    }
}

impl Default for LocalStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// sessionStorage — 会话级键值存储
///
/// 数据在应用关闭后清除，仅存在于内存中。
#[derive(Debug, Clone, Default)]
pub struct SessionStorage {
    backend: MemoryStorage,
}

impl SessionStorage {
    pub fn new() -> Self {
        Self {
            backend: MemoryStorage::new(),
        }
    }

    pub fn length(&self) -> usize {
        self.backend.length()
    }

    pub fn get_item(&self, key: &str) -> Option<String> {
        self.backend.get_item(key)
    }

    pub fn set_item(&mut self, key: &str, value: &str) {
        self.backend.set_item(key, value);
    }

    pub fn remove_item(&mut self, key: &str) {
        self.backend.remove_item(key);
    }

    pub fn clear(&mut self) {
        self.backend.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_storage() {
        let mut storage = MemoryStorage::new();
        assert_eq!(storage.length(), 0);

        storage.set_item("key1", "value1");
        assert_eq!(storage.length(), 1);
        assert_eq!(storage.get_item("key1"), Some("value1".to_string()));

        storage.remove_item("key1");
        assert_eq!(storage.length(), 0);
    }

    #[test]
    fn test_local_storage() {
        let mut ls = LocalStorage::new();
        ls.set_item("theme", "dark");
        assert_eq!(ls.get_item("theme"), Some("dark".to_string()));
        assert_eq!(ls.length(), 1);

        ls.clear();
        assert_eq!(ls.length(), 0);
    }

    #[test]
    fn test_session_storage() {
        let mut ss = SessionStorage::new();
        ss.set_item("token", "abc123");
        assert_eq!(ss.get_item("token"), Some("abc123".to_string()));

        let mut ss2 = SessionStorage::new();
        // 新实例独立
        assert_eq!(ss2.get_item("token"), None);
    }
}
