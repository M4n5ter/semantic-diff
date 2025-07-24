//! semantic-diff-core - 语义代码差异分析核心库
//!
//! 这是一个基于 Tree-sitter 的 Go 语言代码差异分析核心库，
//! 提供语义级别的代码差异分析功能。

pub mod analyzer;
pub mod error;
pub mod extractor;
pub mod formatter;
pub mod generator;
pub mod git;
pub mod parser;
pub mod performance;

// 重新导出主要的公共 API
pub use analyzer::{DependencyResolver, TypeAnalyzer};
pub use error::{Result, SemanticDiffError};
pub use extractor::{SemanticContext, SemanticContextExtractor};
pub use formatter::{
    BlockTitleStyle, ColorTheme, FormattedOutput, FormatterConfig, OutputMetadata, OutputRenderer,
    SyntaxHighlighter,
};
pub use generator::{CodeSlice, CodeSliceGenerator, HighlightStyle, OutputFormat};
pub use git::{ChangeType, DiffHunk, FileChange, GitDiffParser};
// 导出多语言解析器架构
pub use parser::{
    Declaration, GoConstantDefinition, GoDeclaration, GoFunctionInfo, GoLanguageInfo, GoParameter,
    GoParser, GoReceiverInfo, GoType, GoTypeDefinition, GoTypeKind, GoVariableDefinition, Import,
    LanguageParser, LanguageSpecificInfo, ParserFactory, SourceFile, SupportedLanguage,
};
// 导出性能优化组件
pub use performance::{
    CacheStats, ConcurrentFileProcessor, ErrorRecoveryStrategy, MemoryEfficientAstProcessor,
    ParseResult, ParserCache, PerformanceMonitor, PerformanceStats,
};
