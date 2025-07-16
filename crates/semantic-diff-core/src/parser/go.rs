//! Go 语言解析器实现
//!
//! 基于 Tree-sitter 的 Go 语言源码解析器

use super::common::{Declaration, Import, LanguageParser, LanguageSpecificInfo, SupportedLanguage};
use crate::error::{Result, SemanticDiffError};
use std::path::PathBuf;
use tree_sitter::{Node, Parser, Point, Tree};

/// Go 语言函数信息
#[derive(Debug)]
pub struct GoFunctionInfo {
    pub name: String,
    pub receiver: Option<GoReceiverInfo>,
    pub parameters: Vec<GoParameter>,
    pub return_types: Vec<GoType>,
    pub body: String,
    pub start_line: u32,
    pub end_line: u32,
    pub file_path: PathBuf,
}

/// Go 语言接收者信息（方法的接收者）
#[derive(Debug)]
pub struct GoReceiverInfo {
    pub name: String,
    pub type_name: String,
    pub is_pointer: bool,
}

/// Go 语言参数信息
#[derive(Debug)]
pub struct GoParameter {
    pub name: String,
    pub param_type: GoType,
}

/// Go 语言类型信息
#[derive(Debug)]
pub struct GoType {
    pub name: String,
    pub is_pointer: bool,
    pub is_slice: bool,
}

/// Go 语言类型定义
#[derive(Debug)]
pub struct GoTypeDefinition {
    pub name: String,
    pub kind: GoTypeKind,
    pub definition: String,
    pub file_path: PathBuf,
    pub dependencies: Vec<String>,
}

/// Go 语言类型种类
#[derive(Debug)]
pub enum GoTypeKind {
    Struct,
    Interface,
    Alias,
    Enum,
    Constant,
}

/// Go 语言常量定义
#[derive(Debug)]
pub struct GoConstantDefinition {
    pub name: String,
    pub value: String,
    pub const_type: Option<GoType>,
    pub start_line: u32,
    pub end_line: u32,
    pub file_path: PathBuf,
}

/// Go 语言变量定义
#[derive(Debug)]
pub struct GoVariableDefinition {
    pub name: String,
    pub var_type: Option<GoType>,
    pub initial_value: Option<String>,
    pub start_line: u32,
    pub end_line: u32,
    pub file_path: PathBuf,
}

/// Go 语言声明枚举
#[derive(Debug)]
pub enum GoDeclaration {
    Function(GoFunctionInfo),
    Type(GoTypeDefinition),
    Constant(GoConstantDefinition),
    Variable(GoVariableDefinition),
}

impl Declaration for GoDeclaration {
    fn name(&self) -> &str {
        match self {
            GoDeclaration::Function(f) => &f.name,
            GoDeclaration::Type(t) => &t.name,
            GoDeclaration::Constant(c) => &c.name,
            GoDeclaration::Variable(v) => &v.name,
        }
    }

    fn declaration_type(&self) -> &str {
        match self {
            GoDeclaration::Function(_) => "function",
            GoDeclaration::Type(_) => "type",
            GoDeclaration::Constant(_) => "constant",
            GoDeclaration::Variable(_) => "variable",
        }
    }

    fn start_line(&self) -> u32 {
        match self {
            GoDeclaration::Function(f) => f.start_line,
            GoDeclaration::Type(_t) => {
                // 对于类型定义，我们需要从定义中解析行号，这里先返回 0
                // 在实际实现中应该从 AST 中获取
                0
            }
            GoDeclaration::Constant(c) => c.start_line,
            GoDeclaration::Variable(v) => v.start_line,
        }
    }

    fn end_line(&self) -> u32 {
        match self {
            GoDeclaration::Function(f) => f.end_line,
            GoDeclaration::Type(_) => {
                // 对于类型定义，我们需要从定义中解析行号，这里先返回 0
                // 在实际实现中应该从 AST 中获取
                0
            }
            GoDeclaration::Constant(c) => c.end_line,
            GoDeclaration::Variable(v) => v.end_line,
        }
    }

    fn file_path(&self) -> &PathBuf {
        match self {
            GoDeclaration::Function(f) => &f.file_path,
            GoDeclaration::Type(t) => &t.file_path,
            GoDeclaration::Constant(c) => &c.file_path,
            GoDeclaration::Variable(v) => &v.file_path,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Go 语言特定信息
#[derive(Debug)]
pub struct GoLanguageInfo {
    pub package_name: String,
    pub imports: Vec<Import>,
    pub declarations: Vec<Box<dyn Declaration>>,
}

impl GoLanguageInfo {
    /// 创建新的 Go 语言信息实例
    pub fn new(package_name: String) -> Self {
        Self {
            package_name,
            imports: Vec::new(),
            declarations: Vec::new(),
        }
    }

    /// 添加导入声明
    pub fn add_import(&mut self, import: Import) {
        self.imports.push(import);
    }

    /// 添加声明
    pub fn add_declaration(&mut self, declaration: Box<dyn Declaration>) {
        self.declarations.push(declaration);
    }

    /// 添加 Go 特定的声明
    pub fn add_go_declaration(&mut self, declaration: GoDeclaration) {
        self.declarations.push(Box::new(declaration));
    }

    /// 查找指定名称的函数
    pub fn find_function(&self, name: &str) -> Option<&GoFunctionInfo> {
        self.declarations.iter().find_map(|decl| {
            if let Some(go_decl) = decl.as_any().downcast_ref::<GoDeclaration>() {
                if let GoDeclaration::Function(func) = go_decl {
                    if func.name == name { Some(func) } else { None }
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    /// 查找指定名称的类型定义
    pub fn find_type(&self, name: &str) -> Option<&GoTypeDefinition> {
        self.declarations.iter().find_map(|decl| {
            if let Some(go_decl) = decl.as_any().downcast_ref::<GoDeclaration>() {
                if let GoDeclaration::Type(type_def) = go_decl {
                    if type_def.name == name {
                        Some(type_def)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        })
    }
}

impl LanguageSpecificInfo for GoLanguageInfo {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn language(&self) -> SupportedLanguage {
        SupportedLanguage::Go
    }

    fn package_name(&self) -> &str {
        &self.package_name
    }

    fn imports(&self) -> &[Import] {
        &self.imports
    }

    fn declarations(&self) -> &[Box<dyn Declaration>] {
        &self.declarations
    }
}

/// Go 语言解析器
pub struct GoParser {
    parser: Parser,
}

impl GoParser {
    /// 创建新的 Go 解析器
    pub fn new() -> Result<Self> {
        let language = tree_sitter_go::LANGUAGE.into();
        let mut parser = Parser::new();

        parser.set_language(&language).map_err(|e| {
            SemanticDiffError::TreeSitterError(format!("Failed to set Go language: {e}"))
        })?;

        Ok(Self { parser })
    }

    /// 递归查找包含指定位置的最小节点
    fn find_node_at_point<'a>(&self, node: Node<'a>, point: Point) -> Option<Node<'a>> {
        if !node.start_position().le(&point) || !point.le(&node.end_position()) {
            return None;
        }

        // 查找最小的包含该位置的子节点
        for child in node.children(&mut node.walk()) {
            if let Some(found) = self.find_node_at_point(child, point) {
                return Some(found);
            }
        }

        // 如果没有子节点包含该位置，返回当前节点
        Some(node)
    }

    /// 递归遍历语法树并收集节点类型的辅助方法
    fn walk_tree_recursive_collect(
        &self,
        cursor: &mut tree_sitter::TreeCursor,
        node_kinds: &mut Vec<String>,
    ) {
        node_kinds.push(cursor.node().kind().to_string());

        if cursor.goto_first_child() {
            loop {
                self.walk_tree_recursive_collect(cursor, node_kinds);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }
}

impl LanguageParser for GoParser {
    /// 解析源码为语法树
    fn parse_source(&mut self, source: &str) -> Result<Tree> {
        self.parser.parse(source, None).ok_or_else(|| {
            SemanticDiffError::ParseError("Failed to parse Go source code".to_string())
        })
    }

    /// 根据行列位置查找节点
    fn find_node_at_position<'a>(
        &self,
        tree: &'a Tree,
        line: u32,
        column: u32,
    ) -> Option<Node<'a>> {
        let point = Point::new(line as usize, column as usize);
        let root = tree.root_node();
        self.find_node_at_point(root, point)
    }

    /// 获取节点的文本内容
    fn get_node_text<'a>(&self, node: Node, source: &'a str) -> &'a str {
        &source[node.byte_range()]
    }

    /// 遍历语法树并收集节点信息
    fn walk_tree_collect(&self, root: Node) -> Vec<String> {
        let mut node_kinds = Vec::new();
        let mut cursor = root.walk();
        self.walk_tree_recursive_collect(&mut cursor, &mut node_kinds);
        node_kinds
    }

    /// 获取语言名称
    fn language_name(&self) -> &'static str {
        "Go"
    }

    /// 获取支持的文件扩展名
    fn file_extensions(&self) -> &'static [&'static str] {
        &["go"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    /// 测试 GoParser 的初始化
    #[test]
    fn test_go_parser_initialization() {
        let parser = GoParser::new();
        assert!(parser.is_ok(), "GoParser initialization should succeed");

        let parser = parser.unwrap();
        assert_eq!(parser.language_name(), "Go");
        assert_eq!(parser.file_extensions(), &["go"]);
    }

    /// 测试解析简单的 Go 源码
    #[test]
    fn test_parse_simple_go_code() {
        let mut parser = GoParser::new().expect("Failed to create parser");

        let source = r#"
package main

import "fmt"

func main() {
    fmt.Println("Hello, World!")
}
"#;

        let tree = parser.parse_source(source);
        assert!(tree.is_ok(), "Parsing should succeed");

        let tree = tree.unwrap();
        let root = tree.root_node();

        // 验证根节点是 source_file
        assert_eq!(root.kind(), "source_file");

        // 验证没有语法错误
        assert!(!root.has_error(), "Parsed tree should not have errors");
    }

    /// 测试解析包含函数的 Go 源码
    #[test]
    fn test_parse_go_function() {
        let mut parser = GoParser::new().expect("Failed to create parser");

        let source = r#"
package main

func add(a int, b int) int {
    return a + b
}

func multiply(x, y int) int {
    result := x * y
    return result
}
"#;

        let tree = parser.parse_source(source).expect("Failed to parse source");
        let root = tree.root_node();

        // 验证根节点类型
        assert_eq!(root.kind(), "source_file");

        // 查找函数声明
        let mut function_count = 0;
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            if child.kind() == "function_declaration" {
                function_count += 1;
            }
        }

        assert_eq!(
            function_count, 2,
            "Should find exactly 2 function declarations"
        );
    }

    /// 测试根据位置查找节点
    #[test]
    fn test_find_node_at_position() {
        let mut parser = GoParser::new().expect("Failed to create parser");

        let source = r#"package main

func main() {
    x := 42
    println(x)
}
"#;

        let tree = parser.parse_source(source).expect("Failed to parse source");

        // 查找第 4 行第 5 列的节点（应该在 x := 42 这一行）
        let node = parser.find_node_at_position(&tree, 3, 4);
        assert!(
            node.is_some(),
            "Should find a node at the specified position"
        );

        let node = node.unwrap();
        let node_text = parser.get_node_text(node, source);

        // 验证找到的节点包含预期的内容
        assert!(
            node_text.contains("x") || node_text.contains(":=") || node_text.contains("42"),
            "Node text should contain part of the assignment: '{node_text}'"
        );
    }

    /// 测试获取节点文本
    #[test]
    fn test_get_node_text() {
        let mut parser = GoParser::new().expect("Failed to create parser");

        let source = r#"package main

func hello() string {
    return "Hello, World!"
}
"#;

        let tree = parser.parse_source(source).expect("Failed to parse source");
        let root = tree.root_node();

        // 获取整个源文件的文本
        let root_text = parser.get_node_text(root, source);
        assert_eq!(root_text.trim(), source.trim());

        // 查找函数声明并获取其文本
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if child.kind() == "function_declaration" {
                let func_text = parser.get_node_text(child, source);
                assert!(func_text.contains("func hello()"));
                assert!(func_text.contains("return \"Hello, World!\""));
                break;
            }
        }
    }

    /// 测试遍历语法树
    #[test]
    fn test_walk_tree() {
        let mut parser = GoParser::new().expect("Failed to create parser");

        let source = r#"package main

import "fmt"

func main() {
    fmt.Println("test")
}
"#;

        let tree = parser.parse_source(source).expect("Failed to parse source");
        let root = tree.root_node();

        let node_kinds = parser.walk_tree_collect(root);

        // 验证遍历到了预期的节点类型
        assert!(node_kinds.contains(&"source_file".to_string()));
        assert!(node_kinds.contains(&"package_clause".to_string()));
        assert!(node_kinds.contains(&"import_declaration".to_string()));
        assert!(node_kinds.contains(&"function_declaration".to_string()));
    }

    /// 测试解析包含结构体的 Go 源码
    #[test]
    fn test_parse_go_struct() {
        let mut parser = GoParser::new().expect("Failed to create parser");

        let source = r#"
package main

type Person struct {
    Name string
    Age  int
}

func (p Person) Greet() string {
    return "Hello, " + p.Name
}
"#;

        let tree = parser.parse_source(source).expect("Failed to parse source");
        let root = tree.root_node();

        // 查找类型声明和方法声明
        let mut type_count = 0;
        let mut method_count = 0;
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            match child.kind() {
                "type_declaration" => type_count += 1,
                "method_declaration" => method_count += 1,
                _ => {}
            }
        }

        assert_eq!(type_count, 1, "Should find exactly 1 type declaration");
        assert_eq!(method_count, 1, "Should find exactly 1 method declaration");
    }

    /// 测试解析接口的 Go 源码
    #[test]
    fn test_parse_go_interface() {
        let mut parser = GoParser::new().expect("Failed to create parser");

        let source = r#"
package main

type Writer interface {
    Write([]byte) (int, error)
}

type Reader interface {
    Read([]byte) (int, error)
}

type ReadWriter interface {
    Reader
    Writer
}
"#;

        let tree = parser.parse_source(source).expect("Failed to parse source");
        let root = tree.root_node();

        // 查找接口声明
        let mut interface_count = 0;
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            if child.kind() == "type_declaration" {
                // 递归查找 interface_type 节点
                fn find_interface_type(node: tree_sitter::Node) -> bool {
                    if node.kind() == "interface_type" {
                        return true;
                    }
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if find_interface_type(child) {
                            return true;
                        }
                    }
                    false
                }

                if find_interface_type(child) {
                    interface_count += 1;
                }
            }
        }

        assert_eq!(
            interface_count, 3,
            "Should find exactly 3 interface declarations"
        );
    }

    /// 测试解析常量和变量声明的 Go 源码
    #[test]
    fn test_parse_go_constants_and_variables() {
        let mut parser = GoParser::new().expect("Failed to create parser");

        let source = r#"
package main

const (
    MaxSize = 100
    MinSize = 10
)

var (
    GlobalVar string = "global"
    Counter   int    = 0
)

const SingleConst = "single"
var SingleVar = 42
"#;

        let tree = parser.parse_source(source).expect("Failed to parse source");
        let root = tree.root_node();

        // 查找常量和变量声明
        let mut const_count = 0;
        let mut var_count = 0;
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            match child.kind() {
                "const_declaration" => const_count += 1,
                "var_declaration" => var_count += 1,
                _ => {}
            }
        }

        assert_eq!(const_count, 2, "Should find exactly 2 const declarations");
        assert_eq!(var_count, 2, "Should find exactly 2 var declarations");
    }

    /// 测试解析错误的 Go 源码
    #[test]
    fn test_parse_invalid_go_code() {
        let mut parser = GoParser::new().expect("Failed to create parser");

        let invalid_source = r#"
package main

func main( {
    // 缺少右括号，语法错误
    fmt.Println("test"
}
"#;

        let tree = parser
            .parse_source(invalid_source)
            .expect("Parser should still return a tree");
        let root = tree.root_node();

        // 验证树包含错误
        assert!(
            root.has_error(),
            "Parsed tree should contain errors for invalid syntax"
        );
    }

    /// 测试解析复杂的 Go 源码结构
    #[test]
    fn test_parse_complex_go_code() {
        let mut parser = GoParser::new().expect("Failed to create parser");

        let source = r#"
package main

import (
    "fmt"
    "strings"
)

const MaxSize = 100

var GlobalVar string = "global"

type Config struct {
    Host string `json:"host"`
    Port int    `json:"port"`
}

type Handler interface {
    Handle(request string) (string, error)
}

func NewConfig(host string, port int) *Config {
    return &Config{
        Host: host,
        Port: port,
    }
}

func (c *Config) String() string {
    return fmt.Sprintf("%s:%d", c.Host, c.Port)
}

func main() {
    config := NewConfig("localhost", 8080)
    fmt.Println(config.String())
}
"#;

        let tree = parser
            .parse_source(source)
            .expect("Failed to parse complex source");
        let root = tree.root_node();

        // 验证根节点类型
        assert_eq!(root.kind(), "source_file");

        // 统计各种声明类型
        let mut counts = std::collections::HashMap::new();
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            let kind = child.kind();
            *counts.entry(kind.to_string()).or_insert(0) += 1;
        }

        // 验证找到了预期的声明类型
        assert!(counts.contains_key("package_clause"));
        assert!(counts.contains_key("import_declaration"));
        assert!(counts.contains_key("const_declaration"));
        assert!(counts.contains_key("var_declaration"));
        assert!(counts.contains_key("type_declaration"));
        assert!(counts.contains_key("function_declaration"));
        assert!(counts.contains_key("method_declaration"));

        // 验证没有语法错误
        assert!(
            !root.has_error(),
            "Complex Go code should parse without errors"
        );
    }

    /// 测试解析包含泛型的 Go 源码（Go 1.18+）
    #[test]
    fn test_parse_go_generics() {
        let mut parser = GoParser::new().expect("Failed to create parser");

        let source = r#"
package main

type Stack[T any] struct {
    items []T
}

func (s *Stack[T]) Push(item T) {
    s.items = append(s.items, item)
}

func (s *Stack[T]) Pop() (T, bool) {
    if len(s.items) == 0 {
        var zero T
        return zero, false
    }
    item := s.items[len(s.items)-1]
    s.items = s.items[:len(s.items)-1]
    return item, true
}

func NewStack[T any]() *Stack[T] {
    return &Stack[T]{items: make([]T, 0)}
}
"#;

        let tree = parser
            .parse_source(source)
            .expect("Failed to parse generic source");
        let root = tree.root_node();

        // 验证根节点类型
        assert_eq!(root.kind(), "source_file");

        // 查找类型声明和方法声明
        let mut type_count = 0;
        let mut method_count = 0;
        let mut function_count = 0;
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            match child.kind() {
                "type_declaration" => type_count += 1,
                "method_declaration" => method_count += 1,
                "function_declaration" => function_count += 1,
                _ => {}
            }
        }

        assert_eq!(type_count, 1, "Should find exactly 1 type declaration");
        assert_eq!(method_count, 2, "Should find exactly 2 method declarations");
        assert_eq!(
            function_count, 1,
            "Should find exactly 1 function declaration"
        );

        // 验证没有语法错误
        assert!(
            !root.has_error(),
            "Generic Go code should parse without errors"
        );
    }

    /// 测试解析包含嵌套结构的 Go 源码
    #[test]
    fn test_parse_nested_structures() {
        let mut parser = GoParser::new().expect("Failed to create parser");

        let source = r#"
package main

type Address struct {
    Street string
    City   string
    State  string
}

type Person struct {
    Name    string
    Age     int
    Address Address
    Emails  []string
}

type Company struct {
    Name      string
    Employees []Person
    Address   *Address
}

func (p Person) GetFullAddress() string {
    return p.Address.Street + ", " + p.Address.City + ", " + p.Address.State
}
"#;

        let tree = parser
            .parse_source(source)
            .expect("Failed to parse nested source");
        let root = tree.root_node();

        // 验证根节点类型
        assert_eq!(root.kind(), "source_file");

        // 查找结构体声明
        let mut struct_count = 0;
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            if child.kind() == "type_declaration" {
                // 递归查找 struct_type 节点
                fn find_struct_type(node: tree_sitter::Node) -> bool {
                    if node.kind() == "struct_type" {
                        return true;
                    }
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if find_struct_type(child) {
                            return true;
                        }
                    }
                    false
                }

                if find_struct_type(child) {
                    struct_count += 1;
                }
            }
        }

        assert_eq!(struct_count, 3, "Should find exactly 3 struct declarations");

        // 验证没有语法错误
        assert!(
            !root.has_error(),
            "Nested structure Go code should parse without errors"
        );
    }
}
#[cfg(test)]
mod go_data_tests {
    use super::*;

    #[test]
    fn test_go_language_info_creation() {
        // 测试 GoLanguageInfo 创建
        let go_info = GoLanguageInfo::new("main".to_string());
        assert_eq!(go_info.package_name(), "main");
        assert_eq!(go_info.language(), SupportedLanguage::Go);
        assert!(go_info.imports().is_empty());
        assert!(go_info.declarations().is_empty());
    }

    #[test]
    fn test_go_language_info_add_import() {
        // 测试添加导入声明
        let mut go_info = GoLanguageInfo::new("main".to_string());

        let import1 = Import {
            path: "fmt".to_string(),
            alias: None,
        };
        let import2 = Import {
            path: "github.com/example/pkg".to_string(),
            alias: Some("pkg".to_string()),
        };

        go_info.add_import(import1.clone());
        go_info.add_import(import2.clone());

        assert_eq!(go_info.imports().len(), 2);
        assert_eq!(go_info.imports()[0].path, "fmt");
        assert_eq!(go_info.imports()[0].alias, None);
        assert_eq!(go_info.imports()[1].path, "github.com/example/pkg");
        assert_eq!(go_info.imports()[1].alias, Some("pkg".to_string()));
    }

    #[test]
    fn test_go_language_info_add_declaration() {
        // 测试添加声明
        let mut go_info = GoLanguageInfo::new("main".to_string());

        let func_info = GoFunctionInfo {
            name: "testFunc".to_string(),
            receiver: None,
            parameters: vec![],
            return_types: vec![],
            body: "return nil".to_string(),
            start_line: 1,
            end_line: 3,
            file_path: PathBuf::from("test.go"),
        };

        let type_def = GoTypeDefinition {
            name: "TestStruct".to_string(),
            kind: GoTypeKind::Struct,
            definition: "type TestStruct struct {}".to_string(),
            file_path: PathBuf::from("test.go"),
            dependencies: vec![],
        };

        go_info.add_go_declaration(GoDeclaration::Function(func_info));
        go_info.add_go_declaration(GoDeclaration::Type(type_def));

        assert_eq!(go_info.declarations().len(), 2);

        // 测试查找函数
        let found_func = go_info.find_function("testFunc");
        assert!(found_func.is_some());
        assert_eq!(found_func.unwrap().name, "testFunc");

        // 测试查找类型
        let found_type = go_info.find_type("TestStruct");
        assert!(found_type.is_some());
        assert_eq!(found_type.unwrap().name, "TestStruct");

        // 测试查找不存在的函数和类型
        assert!(go_info.find_function("nonExistent").is_none());
        assert!(go_info.find_type("NonExistent").is_none());
    }

    #[test]
    fn test_language_specific_info_trait() {
        // 测试 LanguageSpecificInfo trait 的实现
        let go_info = GoLanguageInfo::new("test".to_string());
        let info: &dyn LanguageSpecificInfo = &go_info;

        assert_eq!(info.language(), SupportedLanguage::Go);
        assert_eq!(info.package_name(), "test");
        assert!(info.imports().is_empty());
        assert!(info.declarations().is_empty());

        // 测试向下转型
        let any_ref = info.as_any();
        let downcast_result = any_ref.downcast_ref::<GoLanguageInfo>();
        assert!(downcast_result.is_some());
        assert_eq!(downcast_result.unwrap().package_name, "test");
    }

    #[test]
    fn test_go_data_structures() {
        // 测试 Go 特定数据结构的创建
        let go_type = GoType {
            name: "string".to_string(),
            is_pointer: false,
            is_slice: false,
        };
        assert_eq!(go_type.name, "string");
        assert!(!go_type.is_pointer);
        assert!(!go_type.is_slice);

        let parameter = GoParameter {
            name: "msg".to_string(),
            param_type: go_type,
        };
        assert_eq!(parameter.name, "msg");
        assert_eq!(parameter.param_type.name, "string");

        let receiver = GoReceiverInfo {
            name: "s".to_string(),
            type_name: "Server".to_string(),
            is_pointer: true,
        };
        assert_eq!(receiver.name, "s");
        assert_eq!(receiver.type_name, "Server");
        assert!(receiver.is_pointer);

        let constant = GoConstantDefinition {
            name: "MaxSize".to_string(),
            value: "100".to_string(),
            const_type: Some(GoType {
                name: "int".to_string(),
                is_pointer: false,
                is_slice: false,
            }),
            start_line: 1,
            end_line: 1,
            file_path: PathBuf::from("constants.go"),
        };
        assert_eq!(constant.name, "MaxSize");
        assert_eq!(constant.value, "100");
        assert_eq!(constant.start_line, 1);

        let variable = GoVariableDefinition {
            name: "counter".to_string(),
            var_type: Some(GoType {
                name: "int".to_string(),
                is_pointer: false,
                is_slice: false,
            }),
            initial_value: Some("0".to_string()),
            start_line: 2,
            end_line: 2,
            file_path: PathBuf::from("variables.go"),
        };
        assert_eq!(variable.name, "counter");
        assert_eq!(variable.initial_value, Some("0".to_string()));
        assert_eq!(variable.start_line, 2);
    }

    #[test]
    fn test_go_type_kind_enum() {
        // 测试 GoTypeKind 枚举
        let struct_kind = GoTypeKind::Struct;
        let interface_kind = GoTypeKind::Interface;
        let alias_kind = GoTypeKind::Alias;
        let enum_kind = GoTypeKind::Enum;
        let constant_kind = GoTypeKind::Constant;

        // 确保所有变体都能正确创建
        match struct_kind {
            GoTypeKind::Struct => (),
            _ => panic!("Expected Struct variant"),
        }

        match interface_kind {
            GoTypeKind::Interface => (),
            _ => panic!("Expected Interface variant"),
        }

        match alias_kind {
            GoTypeKind::Alias => (),
            _ => panic!("Expected Alias variant"),
        }

        match enum_kind {
            GoTypeKind::Enum => (),
            _ => panic!("Expected Enum variant"),
        }

        match constant_kind {
            GoTypeKind::Constant => (),
            _ => panic!("Expected Constant variant"),
        }
    }

    #[test]
    fn test_go_declaration_enum() {
        // 测试 GoDeclaration 枚举
        let func_info = GoFunctionInfo {
            name: "test".to_string(),
            receiver: None,
            parameters: vec![],
            return_types: vec![],
            body: "".to_string(),
            start_line: 1,
            end_line: 1,
            file_path: PathBuf::from("test.go"),
        };

        let type_def = GoTypeDefinition {
            name: "Test".to_string(),
            kind: GoTypeKind::Struct,
            definition: "".to_string(),
            file_path: PathBuf::from("test.go"),
            dependencies: vec![],
        };

        let const_def = GoConstantDefinition {
            name: "TEST".to_string(),
            value: "1".to_string(),
            const_type: None,
            start_line: 1,
            end_line: 1,
            file_path: PathBuf::from("test.go"),
        };

        let var_def = GoVariableDefinition {
            name: "test".to_string(),
            var_type: None,
            initial_value: None,
            start_line: 1,
            end_line: 1,
            file_path: PathBuf::from("test.go"),
        };

        let func_decl = GoDeclaration::Function(func_info);
        let type_decl = GoDeclaration::Type(type_def);
        let const_decl = GoDeclaration::Constant(const_def);
        let var_decl = GoDeclaration::Variable(var_def);

        // 测试模式匹配
        match func_decl {
            GoDeclaration::Function(ref f) => assert_eq!(f.name, "test"),
            _ => panic!("Expected Function declaration"),
        }

        match type_decl {
            GoDeclaration::Type(ref t) => assert_eq!(t.name, "Test"),
            _ => panic!("Expected Type declaration"),
        }

        match const_decl {
            GoDeclaration::Constant(ref c) => assert_eq!(c.name, "TEST"),
            _ => panic!("Expected Constant declaration"),
        }

        match var_decl {
            GoDeclaration::Variable(ref v) => assert_eq!(v.name, "test"),
            _ => panic!("Expected Variable declaration"),
        }
    }

    #[test]
    fn test_declaration_trait_implementation() {
        // 测试 Declaration trait 的实现
        let func_info = GoFunctionInfo {
            name: "testFunction".to_string(),
            receiver: None,
            parameters: vec![],
            return_types: vec![],
            body: "return nil".to_string(),
            start_line: 10,
            end_line: 15,
            file_path: PathBuf::from("test.go"),
        };

        let func_decl = GoDeclaration::Function(func_info);
        let decl: &dyn Declaration = &func_decl;

        assert_eq!(decl.name(), "testFunction");
        assert_eq!(decl.declaration_type(), "function");
        assert_eq!(decl.start_line(), 10);
        assert_eq!(decl.end_line(), 15);
        assert_eq!(decl.file_path(), &PathBuf::from("test.go"));

        // 测试向下转型
        let any_ref = decl.as_any();
        let downcast_result = any_ref.downcast_ref::<GoDeclaration>();
        assert!(downcast_result.is_some());

        if let GoDeclaration::Function(f) = downcast_result.unwrap() {
            assert_eq!(f.name, "testFunction");
        } else {
            panic!("Expected Function declaration");
        }
    }
}
