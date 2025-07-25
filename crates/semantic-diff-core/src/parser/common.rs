//! 通用解析器接口和数据结构
//!
//! 定义多语言解析器的通用接口和共享数据结构

use crate::error::{Result, SemanticDiffError};
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Tree};

/// 支持的编程语言枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

    /// 克隆为 Box<dyn Declaration>
    fn clone_box(&self) -> Box<dyn Declaration>;
}

/// 语言特定信息的 trait
///
/// 用于存储不同编程语言的特定信息，如包名、导入、声明等
pub trait LanguageSpecificInfo: Send + Sync + std::fmt::Debug {
    /// 转换为 Any trait，用于向下转型
    fn as_any(&self) -> &dyn std::any::Any;
    /// 克隆为 Box<dyn LanguageSpecificInfo>
    fn clone_box(&self) -> Box<dyn LanguageSpecificInfo>;
    /// 获取语言类型
    fn language(&self) -> SupportedLanguage;
    /// 获取包或模块名称
    fn package_name(&self) -> &str;
    /// 获取导入列表
    fn imports(&self) -> &[Import];
    /// 获取声明列表
    fn declarations(&self) -> &[Box<dyn Declaration>];
}

/// 函数签名信息
#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub name: String,
    pub parameters: Vec<String>,
    pub return_types: Vec<String>,
    pub receiver: Option<String>,
}

/// 函数信息
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub signature: FunctionSignature,
    pub body: String,
    pub start_line: u32,
    pub end_line: u32,
    pub file_path: PathBuf,
}

/// CST 导航器 - 提供语法树导航和分析功能
pub struct CstNavigator;

impl CstNavigator {
    /// 创建新的 CST 导航器
    pub fn new() -> Self {
        Self
    }

    /// 查找所有函数声明节点
    pub fn find_function_declarations<'a>(&self, root: Node<'a>) -> Vec<Node<'a>> {
        let mut functions = Vec::new();
        Self::find_nodes_by_kind(root, "function_declaration", &mut functions);
        functions
    }

    /// 查找所有方法声明节点
    pub fn find_method_declarations<'a>(&self, root: Node<'a>) -> Vec<Node<'a>> {
        let mut methods = Vec::new();
        Self::find_nodes_by_kind(root, "method_declaration", &mut methods);
        methods
    }

    /// 查找所有类型声明节点
    pub fn find_type_declarations<'a>(&self, root: Node<'a>) -> Vec<Node<'a>> {
        let mut types = Vec::new();
        Self::find_nodes_by_kind(root, "type_declaration", &mut types);
        types
    }

    /// 查找所有导入声明节点
    pub fn find_import_declarations<'a>(&self, root: Node<'a>) -> Vec<Node<'a>> {
        let mut imports = Vec::new();
        Self::find_nodes_by_kind(root, "import_declaration", &mut imports);
        imports
    }

    /// 查找所有常量声明节点
    pub fn find_const_declarations<'a>(&self, root: Node<'a>) -> Vec<Node<'a>> {
        let mut constants = Vec::new();
        Self::find_nodes_by_kind(root, "const_declaration", &mut constants);
        constants
    }

    /// 查找所有变量声明节点
    pub fn find_var_declarations<'a>(&self, root: Node<'a>) -> Vec<Node<'a>> {
        let mut variables = Vec::new();
        Self::find_nodes_by_kind(root, "var_declaration", &mut variables);
        variables
    }

    /// 获取函数的函数体节点
    pub fn get_function_body<'a>(&self, func_node: Node<'a>) -> Option<Node<'a>> {
        // 查找 block 节点作为函数体
        self.find_child_by_kind(func_node, "block")
    }

    /// 提取函数签名信息
    pub fn get_function_signature(
        &self,
        func_node: Node,
        source: &str,
    ) -> Option<FunctionSignature> {
        if func_node.kind() != "function_declaration" && func_node.kind() != "method_declaration" {
            return None;
        }

        let mut name = String::new();
        let mut parameters = Vec::new();
        let mut return_types = Vec::new();
        let mut receiver = None;
        let mut param_lists = Vec::new();

        // 遍历函数节点的子节点
        let mut cursor = func_node.walk();
        for child in func_node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    // 函数声明中的函数名
                    if func_node.kind() == "function_declaration" && name.is_empty() {
                        name = source[child.byte_range()].to_string();
                    }
                }
                "field_identifier" => {
                    // 方法声明中的函数名
                    if func_node.kind() == "method_declaration" && name.is_empty() {
                        name = source[child.byte_range()].to_string();
                    }
                }
                "parameter_list" => {
                    // 收集所有参数列表，第一个是接收者（如果是方法），第二个是参数
                    param_lists.push(child);
                }
                "type_identifier" | "pointer_type" | "slice_type" | "array_type"
                | "qualified_type" => {
                    // 返回类型
                    return_types.push(source[child.byte_range()].to_string());
                }
                _ => {}
            }
        }

        // 处理参数列表
        if func_node.kind() == "method_declaration" {
            // 方法声明：第一个参数列表是接收者，第二个是参数
            if !param_lists.is_empty() {
                receiver = Some(source[param_lists[0].byte_range()].to_string());
            }
            if param_lists.len() >= 2 {
                parameters = self.extract_parameters(param_lists[1], source);
            }
        } else {
            // 函数声明：只有一个参数列表
            if !param_lists.is_empty() {
                parameters = self.extract_parameters(param_lists[0], source);
            }
        }

        Some(FunctionSignature {
            name,
            parameters,
            return_types,
            receiver,
        })
    }

    /// 提取节点中的类型引用
    pub fn extract_type_references(&self, node: Node, source: &str) -> Vec<String> {
        let mut type_refs = Vec::new();
        Self::collect_type_references(node, source, &mut type_refs);
        type_refs
    }

    /// 根据行号查找包含该行的节点
    pub fn find_node_containing_line<'a>(&self, root: Node<'a>, line: u32) -> Option<Node<'a>> {
        self.find_node_containing_line_recursive(root, line)
    }

    /// 查找包含指定行号范围的所有节点
    pub fn find_nodes_in_line_range<'a>(
        &self,
        root: Node<'a>,
        start_line: u32,
        end_line: u32,
    ) -> Vec<Node<'a>> {
        let mut nodes = Vec::new();
        self.collect_nodes_in_range(root, start_line, end_line, &mut nodes);
        nodes
    }

    /// 获取节点的行号范围
    pub fn get_node_line_range(&self, node: Node) -> (u32, u32) {
        let start_line = node.start_position().row as u32;
        let end_line = node.end_position().row as u32;
        (start_line, end_line)
    }

    /// 检查节点是否包含指定行号
    pub fn node_contains_line(&self, node: Node, line: u32) -> bool {
        let (start_line, end_line) = self.get_node_line_range(node);
        line >= start_line && line <= end_line
    }

    /// 递归查找指定类型的节点
    fn find_nodes_by_kind<'a>(node: Node<'a>, kind: &str, results: &mut Vec<Node<'a>>) {
        if node.kind() == kind {
            results.push(node);
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::find_nodes_by_kind(child, kind, results);
        }
    }

    /// 查找直接子节点中指定类型的节点
    fn find_child_by_kind<'a>(&self, node: Node<'a>, kind: &str) -> Option<Node<'a>> {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .find(|&child| child.kind() == kind)
    }

    /// 提取参数列表
    fn extract_parameters(&self, param_list_node: Node, source: &str) -> Vec<String> {
        let mut parameters = Vec::new();
        let mut cursor = param_list_node.walk();

        for child in param_list_node.children(&mut cursor) {
            if child.kind() == "parameter_declaration" {
                parameters.push(source[child.byte_range()].to_string());
            }
        }

        parameters
    }

    /// 递归收集类型引用
    fn collect_type_references(node: Node, source: &str, type_refs: &mut Vec<String>) {
        match node.kind() {
            "type_identifier" | "qualified_type" => {
                let type_name = source[node.byte_range()].to_string();
                if !type_refs.contains(&type_name) {
                    type_refs.push(type_name);
                }
            }
            "pointer_type" | "slice_type" | "array_type" | "map_type" | "channel_type" => {
                // 递归处理复合类型
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    Self::collect_type_references(child, source, type_refs);
                }
            }
            _ => {
                // 继续递归查找
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    Self::collect_type_references(child, source, type_refs);
                }
            }
        }
    }

    /// 递归查找包含指定行号的节点
    fn find_node_containing_line_recursive<'a>(
        &self,
        node: Node<'a>,
        line: u32,
    ) -> Option<Node<'a>> {
        if !self.node_contains_line(node, line) {
            return None;
        }

        // 查找最小的包含该行的子节点
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = self.find_node_containing_line_recursive(child, line) {
                return Some(found);
            }
        }

        // 如果没有子节点包含该行，返回当前节点
        Some(node)
    }

    /// 收集指定行号范围内的节点
    fn collect_nodes_in_range<'a>(
        &self,
        node: Node<'a>,
        start_line: u32,
        end_line: u32,
        results: &mut Vec<Node<'a>>,
    ) {
        let (node_start, node_end) = self.get_node_line_range(node);

        // 检查节点是否与指定范围有交集
        if node_end >= start_line && node_start <= end_line {
            results.push(node);

            // 递归处理子节点
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                self.collect_nodes_in_range(child, start_line, end_line, results);
            }
        }
    }
}

impl Default for CstNavigator {
    fn default() -> Self {
        Self::new()
    }
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

impl Clone for SourceFile {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            source_code: self.source_code.clone(),
            syntax_tree: self.syntax_tree.clone(),
            language: self.language,
            language_specific: self.language_specific.clone_box(),
        }
    }
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

    #[test]
    fn test_function_signature_creation() {
        // 测试函数签名结构
        let signature = FunctionSignature {
            name: "testFunc".to_string(),
            parameters: vec!["a int".to_string(), "b string".to_string()],
            return_types: vec!["int".to_string(), "error".to_string()],
            receiver: Some("s *Server".to_string()),
        };

        assert_eq!(signature.name, "testFunc");
        assert_eq!(signature.parameters.len(), 2);
        assert_eq!(signature.return_types.len(), 2);
        assert_eq!(signature.receiver, Some("s *Server".to_string()));
    }

    #[test]
    fn test_function_info_creation() {
        // 测试函数信息结构
        let signature = FunctionSignature {
            name: "add".to_string(),
            parameters: vec!["a int".to_string(), "b int".to_string()],
            return_types: vec!["int".to_string()],
            receiver: None,
        };

        let func_info = FunctionInfo {
            name: "add".to_string(),
            signature,
            body: "return a + b".to_string(),
            start_line: 1,
            end_line: 3,
            file_path: PathBuf::from("math.go"),
        };

        assert_eq!(func_info.name, "add");
        assert_eq!(func_info.signature.name, "add");
        assert_eq!(func_info.start_line, 1);
        assert_eq!(func_info.end_line, 3);
    }

    #[test]
    fn test_cst_navigator_creation() {
        // 测试 CstNavigator 创建
        let _navigator = CstNavigator::new();
        let _default_navigator = CstNavigator;
    }
}

#[cfg(test)]
mod cst_navigator_tests {
    use super::*;
    use crate::parser::go::GoParser;

    /// 创建测试用的解析器和语法树
    fn create_test_tree(source: &str) -> (GoParser, Tree) {
        let mut parser = GoParser::new().expect("Failed to create parser");
        let tree = parser.parse_source(source).expect("Failed to parse source");
        (parser, tree)
    }

    #[test]
    fn test_find_function_declarations() {
        let source = r#"
package main

func main() {
    println("Hello")
}

func add(a, b int) int {
    return a + b
}

func multiply(x, y int) int {
    return x * y
}
"#;

        let (_parser, tree) = create_test_tree(source);
        let navigator = CstNavigator::new();
        let functions = navigator.find_function_declarations(tree.root_node());

        assert_eq!(functions.len(), 3, "Should find 3 function declarations");

        // 验证找到的都是函数声明节点
        for func_node in functions {
            assert_eq!(func_node.kind(), "function_declaration");
        }
    }

    #[test]
    fn test_find_method_declarations() {
        let source = r#"
package main

type Server struct {
    port int
}

func (s *Server) Start() error {
    return nil
}

func (s Server) GetPort() int {
    return s.port
}
"#;

        let (_parser, tree) = create_test_tree(source);
        let navigator = CstNavigator::new();
        let methods = navigator.find_method_declarations(tree.root_node());

        assert_eq!(methods.len(), 2, "Should find 2 method declarations");

        // 验证找到的都是方法声明节点
        for method_node in methods {
            assert_eq!(method_node.kind(), "method_declaration");
        }
    }

    #[test]
    fn test_find_type_declarations() {
        let source = r#"
package main

type User struct {
    Name string
    Age  int
}

type Handler interface {
    Handle() error
}

type StringAlias string
"#;

        let (_parser, tree) = create_test_tree(source);
        let navigator = CstNavigator::new();
        let types = navigator.find_type_declarations(tree.root_node());

        assert_eq!(types.len(), 3, "Should find 3 type declarations");

        // 验证找到的都是类型声明节点
        for type_node in types {
            assert_eq!(type_node.kind(), "type_declaration");
        }
    }

    #[test]
    fn test_find_import_declarations() {
        let source = r#"
package main

import "fmt"
import "os"

import (
    "strings"
    "net/http"
)
"#;

        let (_parser, tree) = create_test_tree(source);
        let navigator = CstNavigator::new();
        let imports = navigator.find_import_declarations(tree.root_node());

        assert_eq!(imports.len(), 3, "Should find 3 import declarations");

        // 验证找到的都是导入声明节点
        for import_node in imports {
            assert_eq!(import_node.kind(), "import_declaration");
        }
    }

    #[test]
    fn test_find_const_and_var_declarations() {
        let source = r#"
package main

const MaxSize = 100
const MinSize = 10

const (
    DefaultPort = 8080
    DefaultHost = "localhost"
)

var GlobalVar string = "global"
var Counter int = 0

var (
    Logger *log.Logger
    Config *AppConfig
)
"#;

        let (_parser, tree) = create_test_tree(source);
        let navigator = CstNavigator::new();

        let constants = navigator.find_const_declarations(tree.root_node());
        let variables = navigator.find_var_declarations(tree.root_node());

        assert_eq!(constants.len(), 3, "Should find 3 const declarations");
        assert_eq!(variables.len(), 3, "Should find 3 var declarations");

        // 验证节点类型
        for const_node in constants {
            assert_eq!(const_node.kind(), "const_declaration");
        }

        for var_node in variables {
            assert_eq!(var_node.kind(), "var_declaration");
        }
    }

    #[test]
    fn test_get_function_body() {
        let source = r#"
package main

func add(a, b int) int {
    result := a + b
    return result
}
"#;

        let (_parser, tree) = create_test_tree(source);
        let navigator = CstNavigator::new();
        let functions = navigator.find_function_declarations(tree.root_node());

        assert_eq!(functions.len(), 1);

        let func_node = functions[0];
        let body = navigator.get_function_body(func_node);

        assert!(body.is_some(), "Should find function body");
        assert_eq!(body.unwrap().kind(), "block");
    }

    #[test]
    fn test_get_function_signature() {
        let source = r#"
package main

func add(a int, b int) (int, error) {
    return a + b, nil
}

func (s *Server) Start(port int) error {
    return nil
}
"#;

        let (_parser, tree) = create_test_tree(source);
        let navigator = CstNavigator::new();
        let functions = navigator.find_function_declarations(tree.root_node());
        let methods = navigator.find_method_declarations(tree.root_node());

        assert_eq!(functions.len(), 1);
        assert_eq!(methods.len(), 1);

        // 测试函数签名提取
        let func_signature = navigator.get_function_signature(functions[0], source);
        assert!(func_signature.is_some());

        let sig = func_signature.unwrap();
        assert_eq!(sig.name, "add");
        assert_eq!(sig.parameters.len(), 2); // Should have two parameters
        assert_eq!(sig.parameters[0], "a int");
        assert_eq!(sig.parameters[1], "b int");
        assert!(sig.receiver.is_none());

        // 测试方法签名提取
        let method_signature = navigator.get_function_signature(methods[0], source);
        assert!(method_signature.is_some());

        let method_sig = method_signature.unwrap();
        assert_eq!(method_sig.name, "Start");
        assert_eq!(method_sig.parameters.len(), 1);
        assert_eq!(method_sig.parameters[0], "port int");
        assert_eq!(method_sig.return_types.len(), 1);
        assert_eq!(method_sig.return_types[0], "error");
        assert!(method_sig.receiver.is_some());
        assert_eq!(method_sig.receiver.unwrap(), "(s *Server)");
    }

    #[test]
    fn test_extract_type_references() {
        let source = r#"
package main

type User struct {
    Name    string
    Age     int
    Address *Address
    Tags    []string
    Config  map[string]interface{}
}
"#;

        let (_parser, tree) = create_test_tree(source);
        let navigator = CstNavigator::new();
        let types = navigator.find_type_declarations(tree.root_node());

        assert_eq!(types.len(), 1);

        let type_refs = navigator.extract_type_references(types[0], source);

        // Should find some type references
        assert!(!type_refs.is_empty(), "Should find type references");

        // Possible type references (depends on tree-sitter parsing results)
        println!("Found type references: {type_refs:?}");
    }

    #[test]
    fn test_find_node_containing_line() {
        let source = r#"package main

func main() {
    x := 42
    println(x)
}

func add(a, b int) int {
    return a + b
}
"#;

        let (_parser, tree) = create_test_tree(source);
        let navigator = CstNavigator::new();

        // Find node at line 4 (x := 42)
        let node = navigator.find_node_containing_line(tree.root_node(), 3);
        assert!(node.is_some(), "Should find node containing line 4");

        // Find node at line 8 (return a + b)
        let node = navigator.find_node_containing_line(tree.root_node(), 7);
        assert!(node.is_some(), "Should find node containing line 8");

        // Find non-existent line
        let node = navigator.find_node_containing_line(tree.root_node(), 100);
        assert!(node.is_none(), "Should not find node for non-existent line");
    }

    #[test]
    fn test_find_nodes_in_line_range() {
        let source = r#"package main

func main() {
    x := 42
    y := 24
    println(x + y)
}
"#;

        let (_parser, tree) = create_test_tree(source);
        let navigator = CstNavigator::new();

        // Find nodes in line range 3-5
        let nodes = navigator.find_nodes_in_line_range(tree.root_node(), 2, 4);

        assert!(!nodes.is_empty(), "Should find nodes in specified range");

        // Verify all found nodes are within the specified range
        for node in nodes {
            let (start_line, end_line) = navigator.get_node_line_range(node);
            assert!(
                end_line >= 2 && start_line <= 4,
                "Node ({start_line}, {end_line}) should intersect with range (2, 4)"
            );
        }
    }

    #[test]
    fn test_get_node_line_range() {
        let source = r#"package main

func main() {
    println("Hello")
}
"#;

        let (_parser, tree) = create_test_tree(source);
        let navigator = CstNavigator::new();
        let root = tree.root_node();

        let (start_line, end_line) = navigator.get_node_line_range(root);

        // Root node should contain the entire file
        assert_eq!(start_line, 0, "Root node should start at line 0");
        assert!(end_line > 0, "Root node should end after line 0");
    }

    #[test]
    fn test_node_contains_line() {
        let source = r#"package main

func main() {
    println("Hello")
}
"#;

        let (_parser, tree) = create_test_tree(source);
        let navigator = CstNavigator::new();
        let root = tree.root_node();

        // Test lines contained by root node
        assert!(
            navigator.node_contains_line(root, 0),
            "Root node should contain line 0"
        );
        assert!(
            navigator.node_contains_line(root, 2),
            "Root node should contain line 2"
        );

        // Test lines not contained by root node (out of range)
        assert!(
            !navigator.node_contains_line(root, 100),
            "Root node should not contain line 100"
        );
    }

    #[test]
    fn test_complex_go_structure_navigation() {
        let source = r#"
package main

import (
    "fmt"
    "net/http"
)

const (
    DefaultPort = 8080
    MaxConnections = 1000
)

var (
    server *http.Server
    logger *Logger
)

type Config struct {
    Port int    `json:"port"`
    Host string `json:"host"`
}

type Logger interface {
    Log(message string)
    Error(err error)
}

type Server struct {
    config *Config
    logger Logger
}

func NewServer(config *Config, logger Logger) *Server {
    return &Server{
        config: config,
        logger: logger,
    }
}

func (s *Server) Start() error {
    addr := fmt.Sprintf("%s:%d", s.config.Host, s.config.Port)
    return http.ListenAndServe(addr, nil)
}

func (s *Server) Stop() error {
    return nil
}

func main() {
    config := &Config{
        Port: DefaultPort,
        Host: "localhost",
    }
    
    server := NewServer(config, nil)
    if err := server.Start(); err != nil {
        panic(err)
    }
}
"#;

        let (_parser, tree) = create_test_tree(source);
        let navigator = CstNavigator::new();
        let root = tree.root_node();

        // 测试各种声明的查找
        let imports = navigator.find_import_declarations(root);
        let constants = navigator.find_const_declarations(root);
        let variables = navigator.find_var_declarations(root);
        let types = navigator.find_type_declarations(root);
        let functions = navigator.find_function_declarations(root);
        let methods = navigator.find_method_declarations(root);

        assert_eq!(imports.len(), 1, "Should find 1 import declaration block");
        assert_eq!(constants.len(), 1, "Should find 1 const declaration block");
        assert_eq!(variables.len(), 1, "Should find 1 var declaration block");
        assert_eq!(types.len(), 3, "Should find 3 type declarations");
        assert_eq!(functions.len(), 2, "Should find 2 function declarations");
        assert_eq!(methods.len(), 2, "Should find 2 method declarations");

        // Test function body finding
        for func_node in functions {
            let body = navigator.get_function_body(func_node);
            assert!(body.is_some(), "Each function should have a body");
        }

        // Test method body finding
        for method_node in methods {
            let body = navigator.get_function_body(method_node);
            assert!(body.is_some(), "Each method should have a body");
        }

        // Test type reference extraction
        for type_node in types {
            let type_refs = navigator.extract_type_references(type_node, source);
            // Structs and interfaces should have type references
            if !type_refs.is_empty() {
                println!(
                    "Type {} references: {type_refs:?}",
                    source[type_node.byte_range()].lines().next().unwrap_or("")
                );
            }
        }

        // Test line number finding
        let main_func_line = 47; // main function is around line 47
        let node = navigator.find_node_containing_line(root, main_func_line);
        assert!(node.is_some(), "Should find node containing main function");
    }
}
