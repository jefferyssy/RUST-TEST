//! Fetch API —— Phase 3
//!
//! 对应 W3C Fetch Living Standard。
//! Phase 3: Headers/Request 独立可构造 + fetch_async 异步接口。

use std::collections::HashMap;

/// HTTP 请求方法
#[derive(Debug, Clone, PartialEq)]
pub enum FetchMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
    Custom(String),
}

impl FetchMethod {
    pub fn as_str(&self) -> &str {
        match self {
            FetchMethod::Get => "GET",
            FetchMethod::Post => "POST",
            FetchMethod::Put => "PUT",
            FetchMethod::Delete => "DELETE",
            FetchMethod::Patch => "PATCH",
            FetchMethod::Head => "HEAD",
            FetchMethod::Options => "OPTIONS",
            FetchMethod::Custom(s) => s.as_str(),
        }
    }
}

// ============================================================
//  Headers —— 独立可构造的 HTTP 头集合 (Phase 3 新增)
// ============================================================

/// Headers 对象 —— 对应 W3C Headers API
#[derive(Debug, Clone)]
pub struct Headers {
    inner: HashMap<String, String>,
}

impl Headers {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn set(&mut self, name: &str, value: &str) {
        self.inner.insert(name.to_lowercase(), value.to_string());
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.inner.get(&name.to_lowercase()).map(|s| s.as_str())
    }

    pub fn has(&self, name: &str) -> bool {
        self.inner.contains_key(&name.to_lowercase())
    }

    pub fn delete(&mut self, name: &str) {
        self.inner.remove(&name.to_lowercase());
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.inner.iter()
    }

    pub fn content_type(&self) -> Option<&str> {
        self.get("content-type")
    }

    pub fn authorization(&self) -> Option<&str> {
        self.get("authorization")
    }

    pub fn set_content_type(&mut self, value: &str) {
        self.set("content-type", value);
    }

    pub fn set_authorization(&mut self, value: &str) {
        self.set("authorization", value);
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl Default for Headers {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
//  Request —— 独立可构造的请求对象 (Phase 3 新增)
// ============================================================

/// 请求模式
#[derive(Debug, Clone, PartialEq)]
pub enum RequestMode {
    SameOrigin,
    NoCors,
    Cors,
    Navigate,
}

/// 凭据模式
#[derive(Debug, Clone, PartialEq)]
pub enum RequestCredentials {
    Omit,
    SameOrigin,
    Include,
}

/// Request 对象 —— 对应 W3C Request API (Phase 3 独立可构造)
#[derive(Debug, Clone)]
pub struct Request {
    pub url: String,
    pub method: String,
    pub headers: Headers,
    pub body: Option<Vec<u8>>,
    pub mode: RequestMode,
    pub credentials: RequestCredentials,
}

impl Request {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            method: "GET".to_string(),
            headers: Headers::new(),
            body: None,
            mode: RequestMode::Cors,
            credentials: RequestCredentials::SameOrigin,
        }
    }

    pub fn with_method(mut self, method: &str) -> Self {
        self.method = method.to_uppercase();
        self
    }

    pub fn with_headers(mut self, headers: Headers) -> Self {
        self.headers = headers;
        self
    }

    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }

    pub fn with_mode(mut self, mode: RequestMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_credentials(mut self, creds: RequestCredentials) -> Self {
        self.credentials = creds;
        self
    }
}

/// Fetch 请求配置（兼容旧 API）
#[derive(Debug, Clone)]
pub struct FetchRequest {
    pub url: String,
    pub method: FetchMethod,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub redirect: String,
    pub credentials: bool,
}

impl FetchRequest {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            method: FetchMethod::Get,
            headers: HashMap::new(),
            body: None,
            redirect: "follow".to_string(),
            credentials: true,
        }
    }

    pub fn method(mut self, method: FetchMethod) -> Self {
        self.method = method;
        self
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn body(mut self, body: &str) -> Self {
        self.body = Some(body.to_string());
        self
    }
}

// ============================================================
//  FetchOptions —— fetch_async 参数 (Phase 3 新增)
// ============================================================

/// 异步 fetch 请求选项
#[derive(Debug, Clone)]
pub struct FetchOptions {
    pub method: String,
    pub headers: Headers,
    pub body: Option<Vec<u8>>,
    pub mode: RequestMode,
    pub credentials: RequestCredentials,
}

impl FetchOptions {
    pub fn new() -> Self {
        Self {
            method: "GET".to_string(),
            headers: Headers::new(),
            body: None,
            mode: RequestMode::Cors,
            credentials: RequestCredentials::SameOrigin,
        }
    }

    pub fn method(mut self, method: &str) -> Self {
        self.method = method.to_uppercase();
        self
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.set(name, value);
        self
    }

    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Fetch 响应
#[derive(Debug, Clone)]
pub struct FetchResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub ok: bool,
}

impl FetchResponse {
    pub fn ok_with(body: &str) -> Self {
        Self {
            status: 200,
            status_text: "OK".to_string(),
            headers: HashMap::new(),
            body: Some(body.to_string()),
            ok: true,
        }
    }

    pub fn error(status: u16, text: &str) -> Self {
        Self {
            status,
            status_text: text.to_string(),
            headers: HashMap::new(),
            body: None,
            ok: false,
        }
    }

    /// 将响应体解析为 JSON
    pub fn json(&self) -> Result<String, FetchError> {
        Ok(self.body.clone().unwrap_or_else(|| "{}".to_string()))
    }

    /// 将响应体解析为文本
    pub fn text(&self) -> Result<String, FetchError> {
        Ok(self.body.clone().unwrap_or_default())
    }
}

/// Fetch 错误
#[derive(Debug, Clone)]
pub enum FetchError {
    NetworkError(String),
    Timeout,
    Aborted,
    ParseError(String),
}

/// 发送同步 fetch 请求
///
/// Phase 3+: 使用 reqwest 执行真实 HTTP 请求。
pub fn fetch(_request: &FetchRequest) -> Result<FetchResponse, FetchError> {
    Ok(FetchResponse::ok_with(""))
}

// ============================================================
//  fetch_async —— 异步 fetch (Phase 3 新增)
// ============================================================

/// 异步 fetch 请求
///
/// Phase 3: 提供异步接口签名，内部暂用同步实现（无 tokio 依赖）。
/// Phase 3+: 集成 reqwest 提供真实异步 HTTP。
pub async fn fetch_async(url: &str, options: FetchOptions) -> Result<FetchResponse, FetchError> {
    let method = if options.method == "GET" {
        FetchMethod::Get
    } else if options.method == "POST" {
        FetchMethod::Post
    } else if options.method == "PUT" {
        FetchMethod::Put
    } else if options.method == "DELETE" {
        FetchMethod::Delete
    } else {
        FetchMethod::Custom(options.method.clone())
    };

    let mut req = FetchRequest::new(url).method(method);
    for (k, v) in options.headers.iter() {
        req = req.header(k, v);
    }
    if let Some(body) = &options.body {
        if let Ok(s) = String::from_utf8(body.clone()) {
            req = req.body(&s);
        }
    }

    fetch(&req)
}

impl From<&Request> for FetchRequest {
    fn from(req: &Request) -> Self {
        let mut fetch_req = FetchRequest::new(&req.url);
        if req.method != "GET" {
            fetch_req.method = match req.method.as_str() {
                "POST" => FetchMethod::Post,
                "PUT" => FetchMethod::Put,
                "DELETE" => FetchMethod::Delete,
                "PATCH" => FetchMethod::Patch,
                "HEAD" => FetchMethod::Head,
                "OPTIONS" => FetchMethod::Options,
                other => FetchMethod::Custom(other.to_string()),
            };
        }
        for (k, v) in req.headers.iter() {
            fetch_req.headers.insert(k.clone(), v.clone());
        }
        if let Some(body) = &req.body {
            if let Ok(s) = String::from_utf8(body.clone()) {
                fetch_req.body = Some(s);
            }
        }
        fetch_req
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_request_builder() {
        let req = FetchRequest::new("https://example.com")
            .method(FetchMethod::Post)
            .header("Content-Type", "application/json")
            .body(r#"{"key":"value"}"#);
        assert_eq!(req.url, "https://example.com");
        assert_eq!(req.method, FetchMethod::Post);
        assert!(req.body.is_some());
    }

    #[test]
    fn test_fetch_stub() {
        let req = FetchRequest::new("/api/data");
        let resp = fetch(&req).unwrap();
        assert!(resp.ok);
        assert_eq!(resp.status, 200);
    }
}
