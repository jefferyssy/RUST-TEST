//! WebSocket —— Phase 3
//!
//! 对应 W3C WebSocket API。
//! Phase 3: 自动重连、心跳、连接状态跟踪。

/// WebSocket 连接状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WebSocketState {
    Connecting,
    Open,
    Closing,
    Closed,
}

/// WebSocket 事件
#[derive(Debug, Clone)]
pub enum WebSocketEvent {
    Open,
    Message(String),
    Error(String),
    Close { code: u16, reason: String },
}

// ============================================================
//  WebSocketConfig —— 连接配置 (Phase 3 新增)
// ============================================================

/// WebSocket 连接配置
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    /// 断开后自动重连
    pub auto_reconnect: bool,
    /// 最大重连次数（0 = 无限）
    pub max_reconnect_attempts: u32,
    /// 重连退避基数（毫秒）
    pub reconnect_base_delay_ms: u64,
    /// 最大重连延迟（毫秒）
    pub reconnect_max_delay_ms: u64,
    /// PING 间隔（毫秒，0 = 不发送心跳）
    pub ping_interval_ms: u64,
    /// PONG 超时（毫秒，超时后断开重连）
    pub pong_timeout_ms: u64,
    /// Per-Message Deflate 压缩
    pub compression: bool,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            auto_reconnect: true,
            max_reconnect_attempts: 5,
            reconnect_base_delay_ms: 1000,
            reconnect_max_delay_ms: 30000,
            ping_interval_ms: 30000,
            pong_timeout_ms: 10000,
            compression: false,
        }
    }
}

// ============================================================
//  ConnectionState —— 增强的连接状态 (Phase 3 新增)
// ============================================================

/// 增强的连接状态（含重连信息）
#[derive(Debug, Clone)]
pub enum ConnectionState {
    Connecting,
    Connected,
    Reconnecting { attempt: u32, next_delay_ms: u64 },
    Closed,
}

/// WebSocket 客户端
pub struct WebSocket {
    pub url: String,
    pub state: WebSocketState,
    #[allow(dead_code)]
    config: WebSocketConfig,
    reconnect_attempts: u32,
    last_ping_time: Option<std::time::Instant>,
}

impl WebSocket {
    /// 创建 WebSocket 连接（使用默认配置）
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            state: WebSocketState::Connecting,
            config: WebSocketConfig::default(),
            reconnect_attempts: 0,
            last_ping_time: None,
        }
    }

    /// 使用自定义配置创建 WebSocket (Phase 3 新增)
    pub fn with_config(url: &str, config: WebSocketConfig) -> Self {
        Self {
            url: url.to_string(),
            state: WebSocketState::Connecting,
            config,
            reconnect_attempts: 0,
            last_ping_time: None,
        }
    }

    /// 获取当前状态
    pub fn ready_state(&self) -> WebSocketState {
        self.state
    }

    /// 获取增强的连接状态 (Phase 3 新增)
    pub fn connection_state(&self) -> ConnectionState {
        match self.state {
            WebSocketState::Connecting => {
                if self.reconnect_attempts > 0 {
                    let delay = self.compute_reconnect_delay();
                    ConnectionState::Reconnecting {
                        attempt: self.reconnect_attempts,
                        next_delay_ms: delay,
                    }
                } else {
                    ConnectionState::Connecting
                }
            }
            WebSocketState::Open => ConnectionState::Connected,
            WebSocketState::Closed => ConnectionState::Closed,
            WebSocketState::Closing => ConnectionState::Closed,
        }
    }

    /// 发送文本消息
    pub fn send(&mut self, _data: &str) {
        if self.state == WebSocketState::Open {
            // Phase 3+: 发送帧到 WebSocket 连接
        }
    }

    /// 发送 PING 帧 (Phase 3 新增)
    pub fn ping(&mut self) -> Result<(), WebSocketError> {
        if self.state != WebSocketState::Open {
            return Err(WebSocketError::NotConnected);
        }
        self.last_ping_time = Some(std::time::Instant::now());
        Ok(())
    }

    /// 手动触发重连 (Phase 3 新增)
    pub fn reconnect(&mut self) -> Result<(), WebSocketError> {
        if !self.config.auto_reconnect {
            return Err(WebSocketError::ReconnectDisabled);
        }
        if self.config.max_reconnect_attempts > 0
            && self.reconnect_attempts >= self.config.max_reconnect_attempts
        {
            return Err(WebSocketError::MaxReconnectAttempts);
        }
        self.reconnect_attempts += 1;
        self.state = WebSocketState::Connecting;
        Ok(())
    }

    /// 关闭连接
    pub fn close(&mut self, _code: u16, _reason: &str) {
        self.state = WebSocketState::Closing;
    }

    /// 模拟连接建立
    pub fn simulate_open(&mut self) {
        self.state = WebSocketState::Open;
        self.reconnect_attempts = 0;
    }

    /// 获取当前重连尝试次数
    pub fn reconnect_attempts(&self) -> u32 {
        self.reconnect_attempts
    }

    /// 获取配置
    pub fn config(&self) -> &WebSocketConfig {
        &self.config
    }

    /// 计算下次重连延迟（指数退避）
    fn compute_reconnect_delay(&self) -> u64 {
        let delay = self.config.reconnect_base_delay_ms * 2u64.pow(self.reconnect_attempts.saturating_sub(1));
        delay.min(self.config.reconnect_max_delay_ms)
    }
}

// ============================================================
//  WebSocketError (Phase 3 新增)
// ============================================================

/// WebSocket 错误类型
#[derive(Debug, Clone)]
pub enum WebSocketError {
    NotConnected,
    ReconnectDisabled,
    MaxReconnectAttempts,
    Timeout,
    ProtocolError(String),
}
