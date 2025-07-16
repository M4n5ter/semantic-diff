//! 通用解析器接口和数据结构
//!
//! 定义多语言解析器的通用接口和共享数据结构

use crate::error::{Result, SemanticDiffError};
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Tree};

/// 支持的编程语言枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedLanguage {
    Go,
    // 未来支持的语言
    // Rust,
    // TypeScript,
    // Python,
}

/// 通用语言解析器接口
pub trait LanguageParser: Send + Sync {
    /// 解析源码为语法树
    fn parse_source(&mut self, source: &str) -> Result<Tree>;

    /// 根据行列位置查找节点
    fn find_node_at_position<'a>(&self, tree: &'a Tree, line: u32, column: u32)
    -> Option<Node<'a>>;

    /// 获取节点的文本内容
    fn get_node_text<'a>(&self, node: Node, source: &'a str) -> &'a str;

    /// 遍历语法树并收集节点信息
    fn walk_tree_collect(&self, root: Node) -> Vec<String>;

    /// 获取语言名称
    fn language_name(&self) -> &'static str;

    /// 获取支持的文件扩展名
    fn file_extensions(&self) -> &'static [&'static str];
}

/// 解析器工厂
pub struct ParserFactory;

impl ParserFactory {
    /// 根据语言类型创建解析器
    pub fn create_parser(language: SupportedLanguage) -> Result<Box<dyn LanguageParser>> {
        match language {
            SupportedLanguage::Go => Ok(Box::new(super::go::GoParser::new()?)),
        }
    }

    /// 根据文件路径检测语言类型
    pub fn detect_language(file_path: &Path) -> Option<SupportedLanguage> {
        match file_path.extension()?.to_str()? {
            "go" => Some(SupportedLanguage::Go),
            _ => None,
        }
    }

    /// 根据文件路径创建对应的解析器
    pub fn create_parser_for_file(file_path: &Path) -> Result<Box<dyn LanguageParser>> {
        let language = Self::detect_language(file_path).ok_or_else(|| {
            SemanticDiffError::UnsupportedFileType(file_path.to_string_lossy().to_string())
        })?;
        Self::create_parser(language)
    }
}

/// 通用导入声明
#[derive(Debug, Clone)]
pub struct Import {
    pub path: String,
    pub alias: Option<String>,
}

/// 通用声明类型 - 使用 trait object 来支持不同语言的声明
pub trait Declaration: Send + Sync + std::fmt::Debug {
    /// 获取声明的名称
    fn name(&self) -> &str;

    /// 获取声明的类型（函数、类型、常量等）
    fn declaration_type(&self) -> &str;

    /// 获取声明的起始行
    fn start_line(&self) -> u32;

    /// 获取声明的结束行
    fn end_line(&self) -> u32;

    /// 获取声明所在的文件路径
    fn file_path(&self) -> &PathBuf;

    /// 转换为 Any trait，用于向下转型
    fn as_any(&self) -> &dyn std::any::Any;
}

/// 语言特定信息的 trait
///
/// 用于存储不同编程语言的特定信息，如包名、导入、声明等
pub trait LanguageSpecificInfo: Send + Sync + std::fmt::Debug {
    /// 转换为 Any trait，用于向下转型
    fn as_any(&self) -> &dyn std::any::Any;

    /// 获取语言类型
    fn language(&self) -> SupportedLanguage;

    /// 获取包或模块名称
    fn package_name(&self) -> &str;

    /// 获取导入列表
    fn imports(&self) -> &[Import];

    /// 获取声明列表
    fn declarations(&self) -> &[Box<dyn Declaration>];
}

/// 源文件信息
#[derive(Debug)]
pub struct SourceFile {
    pub path: PathBuf,
    pub source_code: String,
    pub syntax_tree: Tree,
    pub language: SupportedLanguage,
    /// 语言特定的信息通过 trait object 处理
    pub language_specific: Box<dyn LanguageSpecificInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection() {
        // 测试 Go 文件检测
        let go_file = PathBuf::from("main.go");
        assert_eq!(
            ParserFactory::detect_language(&go_file),
            Some(SupportedLanguage::Go)
        );

        // 测试不支持的文件类型
        let unknown_file = PathBuf::from("file.txt");
        assert_eq!(ParserFactory::detect_language(&unknown_file), None);

        // 测试没有扩展名的文件
        let no_ext_file = PathBuf::from("README");
        assert_eq!(ParserFactory::detect_language(&no_ext_file), None);
    }

    #[test]
    fn test_parser_creation() {
        // 测试 Go 解析器创建
        let parser = ParserFactory::create_parser(SupportedLanguage::Go);
        assert!(parser.is_ok());

        let parser = parser.unwrap();
        assert_eq!(parser.language_name(), "Go");
        assert_eq!(parser.file_extensions(), &["go"]);
    }

    #[test]
    fn test_parser_creation_for_file() {
        // 测试根据文件路径创建解析器
        let go_file = PathBuf::from("test.go");
        let parser = ParserFactory::create_parser_for_file(&go_file);
        assert!(parser.is_ok());

        // 测试不支持的文件类型
        let unknown_file = PathBuf::from("test.txt");
        let parser = ParserFactory::create_parser_for_file(&unknown_file);
        assert!(parser.is_err());

        if let Err(SemanticDiffError::UnsupportedFileType(path)) = parser {
            assert_eq!(path, "test.txt");
        } else {
            panic!("Expected UnsupportedFileType error");
        }
    }

    #[test]
    fn test_import_structure() {
        // 测试通用导入结构
        let import = Import {
            path: "fmt".to_string(),
            alias: Some("f".to_string()),
        };
        assert_eq!(import.path, "fmt");
        assert_eq!(import.alias, Some("f".to_string()));

        let import_no_alias = Import {
            path: "os".to_string(),
            alias: None,
        };
        assert_eq!(import_no_alias.path, "os");
        assert_eq!(import_no_alias.alias, None);
    }
}
