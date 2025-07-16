//! semantic-diff-core - 语义代码差异分析核心库
//!
//! 这是一个基于 Tree-sitter 的 Go 语言代码差异分析核心库，
//! 提供语义级别的代码差异分析功能。

pub mod analyzer;
pub mod error;
pub mod extractor;
pub mod generator;
pub mod git;
pub mod parser;

// 重新导出主要的公共 API
pub use analyzer::{DependencyResolver, TypeAnalyzer};
pub use error::{Result, SemanticDiffError};
pub use extractor::{SemanticContext, SemanticContextExtractor};
pub use generator::{CodeSlice, CodeSliceGenerator, HighlightStyle, OutputFormat};
pub use git::{ChangeType, DiffHunk, FileChange, GitDiffParser};
// 导出多语言解析器架构
pub use parser::{
    Declaration, GoConstantDefinition, GoDeclaration, GoFunctionInfo, GoLanguageInfo, GoParameter,
    GoParser, GoReceiverInfo, GoType, GoTypeDefinition, GoTypeKind, GoVariableDefinition, Import,
    LanguageParser, LanguageSpecificInfo, ParserFactory, SourceFile, SupportedLanguage,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_structure() {
        // 基础结构测试，确保模块可以正确导入
        let _error = SemanticDiffError::ConfigError("test".to_string());
        // 测试错误类型可以正确创建
    }
}
