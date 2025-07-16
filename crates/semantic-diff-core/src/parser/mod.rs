//! 多语言解析器模块
//!
//! 提供通用的语言解析器接口和具体的语言实现

pub mod common;
pub mod go;

// 重新导出核心类型
pub use common::{
    Declaration, Import, LanguageParser, LanguageSpecificInfo, ParserFactory, SourceFile,
    SupportedLanguage,
};
pub use go::{
    GoConstantDefinition, GoDeclaration, GoFunctionInfo, GoLanguageInfo, GoParameter, GoParser,
    GoReceiverInfo, GoType, GoTypeDefinition, GoTypeKind, GoVariableDefinition,
};
