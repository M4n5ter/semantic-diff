use thiserror::Error;

/// semantic-diff 工具的错误类型定义
#[derive(Error, Debug)]
pub enum SemanticDiffError {
    #[error("Git repository error: {0}")]
    GitError(String),

    #[error("Go source parsing error: {0}")]
    ParseError(String),

    #[error("File I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid commit hash: {0}")]
    InvalidCommitHash(String),

    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),

    #[error("Dependency resolution failed: {0}")]
    DependencyError(String),

    #[error("Tree-sitter parsing failed: {0}")]
    TreeSitterError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// 项目通用的 Result 类型别名
pub type Result<T> = std::result::Result<T, SemanticDiffError>;
