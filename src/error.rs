use std::fmt;

/// 应用级错误类型
#[derive(Debug)]
pub enum AppError {
    /// 数据库错误
    Database(rusqlite::Error),
    /// IO 错误
    Io(std::io::Error),
    /// 配置序列化/反序列化错误
    Config(String),
    /// 音频解码错误
    Audio(String),
    /// 元数据解析错误
    Metadata(String),
    /// 通用错误
    Other(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Database(e) => write!(f, "数据库错误: {}", e),
            AppError::Io(e) => write!(f, "IO 错误: {}", e),
            AppError::Config(e) => write!(f, "配置错误: {}", e),
            AppError::Audio(e) => write!(f, "音频错误: {}", e),
            AppError::Metadata(e) => write!(f, "元数据错误: {}", e),
            AppError::Other(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for AppError {}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Database(e)
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Config(e.to_string())
    }
}

/// 非致命错误的处理：打印警告并继续
pub fn warn_if_err<T, E: std::fmt::Display>(result: Result<T, E>, context: &str) {
    if let Err(e) = result {
        eprintln!("[WARN] {}: {}", context, e);
    }
}

/// 致命错误的处理：打印错误
pub fn log_error<E: std::fmt::Display>(error: &E, context: &str) {
    eprintln!("[ERROR] {}: {}", context, error);
}
