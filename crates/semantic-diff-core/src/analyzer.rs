//! 代码分析模块
//!
//! 提供依赖关系解析和语义分析功能

use crate::error::{Result, SemanticDiffError};
use crate::git::DiffHunk;
use crate::parser::{
    GoFunctionInfo, GoTypeDefinition, LanguageParser, ParserFactory, SourceFile, SupportedLanguage,
    common::{CstNavigator, LanguageSpecificInfo},
};
use std::fs;
use std::path::Path;

/// 通用源文件分析器
///
/// 提供多语言源文件分析功能，整合解析器和 CST 导航
pub struct SourceAnalyzer {
    parser: Box<dyn LanguageParser>,
    navigator: CstNavigator,
    language: SupportedLanguage,
}

/// 依赖关系解析器
pub struct DependencyResolver;

/// 类型分析器
pub struct TypeAnalyzer;

/// 依赖关系信息
pub struct Dependency {
    pub name: String,
    pub dependency_type: DependencyType,
    pub file_path: std::path::PathBuf,
}

/// 依赖类型
pub enum DependencyType {
    Function,
    Type,
    Constant,
    Variable,
}

/// 类型引用
pub struct TypeReference {
    pub name: String,
    pub package: Option<String>,
}

/// 函数调用
pub struct FunctionCall {
    pub name: String,
    pub receiver: Option<String>,
    pub package: Option<String>,
}

impl Default for DependencyResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyResolver {
    /// 创建新的依赖解析器
    pub fn new() -> Self {
        // TODO: 在任务 6 中实现
        todo!("Implementation in task 6")
    }

    /// 解析类型引用
    pub fn resolve_type(
        &self,
        _type_ref: &TypeReference,
        _source_files: &[SourceFile],
    ) -> Option<GoTypeDefinition> {
        // TODO: 在任务 6 中实现
        todo!("Implementation in task 6")
    }

    /// 解析函数调用
    pub fn resolve_function(
        &self,
        _func_call: &FunctionCall,
        _source_files: &[SourceFile],
    ) -> Option<GoFunctionInfo> {
        // TODO: 在任务 6 中实现
        todo!("Implementation in task 6")
    }
}

impl SourceAnalyzer {
    /// 根据文件路径创建分析器
    ///
    /// 自动检测文件的编程语言并创建对应的解析器
    pub fn new_for_file(file_path: &Path) -> Result<Self> {
        let language = ParserFactory::detect_language(file_path).ok_or_else(|| {
            SemanticDiffError::UnsupportedFileType(file_path.to_string_lossy().to_string())
        })?;

        let parser = ParserFactory::create_parser(language)?;
        let navigator = CstNavigator::new();

        Ok(Self {
            parser,
            navigator,
            language,
        })
    }

    /// 根据语言类型创建分析器
    ///
    /// 直接指定编程语言类型创建解析器
    pub fn new_for_language(language: SupportedLanguage) -> Result<Self> {
        let parser = ParserFactory::create_parser(language)?;
        let navigator = CstNavigator::new();

        Ok(Self {
            parser,
            navigator,
            language,
        })
    }

    /// 分析文件的核心功能
    ///
    /// 读取文件内容，解析为语法树，并提取语言特定信息
    pub fn analyze_file(&mut self, file_path: &Path) -> Result<SourceFile> {
        // 读取文件内容
        let source_code = fs::read_to_string(file_path).map_err(|e| {
            SemanticDiffError::IoError(std::io::Error::new(
                e.kind(),
                format!("Failed to read file {}: {}", file_path.display(), e),
            ))
        })?;

        // 解析源码为语法树
        let syntax_tree = self.parser.parse_source(&source_code)?;

        // 根据语言类型提取特定信息
        let language_specific = match self.language {
            SupportedLanguage::Go => {
                self.extract_go_specific_info(&syntax_tree, &source_code, file_path)?
            } // 未来可以在这里添加其他语言的支持
        };

        Ok(SourceFile {
            path: file_path.to_path_buf(),
            source_code,
            syntax_tree,
            language: self.language,
            language_specific,
        })
    }

    /// 查找变更函数的功能
    ///
    /// 根据差异块信息查找受影响的函数，支持多语言
    pub fn find_changed_functions(
        &self,
        source_file: &SourceFile,
        hunks: &[DiffHunk],
    ) -> Result<Vec<GoFunctionInfo>> {
        let mut changed_functions = Vec::new();

        // 遍历所有差异块
        for hunk in hunks {
            // 获取变更的行号范围
            let start_line = hunk.new_start;
            let end_line = hunk.new_start + hunk.new_lines;

            // 查找包含这些行的函数
            for line in start_line..=end_line {
                if let Some(func_info) = self.find_function_containing_line(source_file, line)? {
                    // 避免重复添加同一个函数
                    if !changed_functions.iter().any(|f: &GoFunctionInfo| {
                        f.name == func_info.name && f.start_line == func_info.start_line
                    }) {
                        changed_functions.push(func_info);
                    }
                }
            }
        }

        Ok(changed_functions)
    }

    /// 查找包含指定行号的函数
    fn find_function_containing_line(
        &self,
        source_file: &SourceFile,
        line: u32,
    ) -> Result<Option<GoFunctionInfo>> {
        let root = source_file.syntax_tree.root_node();

        // 查找包含该行的节点
        if let Some(containing_node) = self.navigator.find_node_containing_line(root, line) {
            // 向上遍历找到函数声明节点
            let mut current_node = containing_node;
            loop {
                if current_node.kind() == "function_declaration"
                    || current_node.kind() == "method_declaration"
                {
                    // 找到函数声明，提取函数信息
                    return Ok(Some(self.extract_function_info(
                        current_node,
                        &source_file.source_code,
                        &source_file.path,
                    )?));
                }

                // 向上查找父节点
                if let Some(parent) = current_node.parent() {
                    current_node = parent;
                } else {
                    break;
                }
            }
        }

        Ok(None)
    }

    /// 提取函数信息
    fn extract_function_info(
        &self,
        func_node: tree_sitter::Node,
        source_code: &str,
        file_path: &Path,
    ) -> Result<GoFunctionInfo> {
        use crate::parser::{GoParameter, GoReceiverInfo, GoType};

        // 获取函数签名
        let signature = self
            .navigator
            .get_function_signature(func_node, source_code)
            .ok_or_else(|| {
                SemanticDiffError::ParseError("Failed to extract function signature".to_string())
            })?;

        // 获取函数体
        let body = if let Some(body_node) = self.navigator.get_function_body(func_node) {
            source_code[body_node.byte_range()].to_string()
        } else {
            String::new()
        };

        // 获取行号范围
        let (start_line, end_line) = self.navigator.get_node_line_range(func_node);

        // 转换参数
        let parameters: Vec<GoParameter> = signature
            .parameters
            .iter()
            .enumerate()
            .map(|(i, param_str)| GoParameter {
                name: format!("param{i}"), // 简化的参数名
                param_type: GoType {
                    name: param_str.clone(),
                    is_pointer: param_str.contains('*'),
                    is_slice: param_str.contains("[]"),
                },
            })
            .collect();

        // 转换返回类型
        let return_types: Vec<GoType> = signature
            .return_types
            .iter()
            .map(|ret_str| GoType {
                name: ret_str.clone(),
                is_pointer: ret_str.contains('*'),
                is_slice: ret_str.contains("[]"),
            })
            .collect();

        // 转换接收者信息
        let receiver = signature.receiver.map(|recv_str| {
            let is_pointer = recv_str.contains('*');
            let type_name = recv_str.replace(['*', '(', ')'], "").trim().to_string();

            GoReceiverInfo {
                name: "self".to_string(), // 简化的接收者名
                type_name,
                is_pointer,
            }
        });

        Ok(GoFunctionInfo {
            name: signature.name.clone(),
            receiver,
            parameters,
            return_types,
            body,
            start_line,
            end_line,
            file_path: file_path.to_path_buf(),
        })
    }

    /// 提取 Go 语言特定信息
    fn extract_go_specific_info(
        &self,
        syntax_tree: &tree_sitter::Tree,
        source_code: &str,
        file_path: &Path,
    ) -> Result<Box<dyn LanguageSpecificInfo>> {
        use crate::parser::GoLanguageInfo;

        let root = syntax_tree.root_node();

        // 提取包名
        let package_name = self.extract_package_name(root, source_code);

        // 提取导入
        let imports = self.extract_imports(root, source_code);

        // 提取声明
        let declarations = self.extract_go_declarations(root, source_code, file_path)?;

        Ok(Box::new(GoLanguageInfo {
            package_name,
            imports,
            declarations,
        }))
    }

    /// 提取 Go 包名
    fn extract_package_name(&self, root: tree_sitter::Node, source_code: &str) -> String {
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if child.kind() == "package_clause" {
                // 查找包名标识符
                let mut pkg_cursor = child.walk();
                for pkg_child in child.children(&mut pkg_cursor) {
                    if pkg_child.kind() == "package_identifier" {
                        return source_code[pkg_child.byte_range()].to_string();
                    }
                }
            }
        }
        "main".to_string() // 默认包名
    }

    /// 提取导入声明
    fn extract_imports(
        &self,
        root: tree_sitter::Node,
        source_code: &str,
    ) -> Vec<crate::parser::Import> {
        let mut imports = Vec::new();
        let import_nodes = self.navigator.find_import_declarations(root);

        for import_node in import_nodes {
            let mut cursor = import_node.walk();
            for child in import_node.children(&mut cursor) {
                match child.kind() {
                    "import_spec" => {
                        // 单个导入规范
                        let import = self.extract_single_import(child, source_code);
                        if let Some(imp) = import {
                            imports.push(imp);
                        }
                    }
                    "import_spec_list" => {
                        // 导入规范列表
                        let mut spec_cursor = child.walk();
                        for spec_child in child.children(&mut spec_cursor) {
                            if spec_child.kind() == "import_spec" {
                                let import = self.extract_single_import(spec_child, source_code);
                                if let Some(imp) = import {
                                    imports.push(imp);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        imports
    }

    /// 提取单个导入
    fn extract_single_import(
        &self,
        import_spec: tree_sitter::Node,
        source_code: &str,
    ) -> Option<crate::parser::Import> {
        let mut path = String::new();
        let mut alias = None;

        let mut cursor = import_spec.walk();
        for child in import_spec.children(&mut cursor) {
            match child.kind() {
                "interpreted_string_literal" => {
                    // 导入路径
                    let path_str = source_code[child.byte_range()].to_string();
                    // 移除引号
                    path = path_str.trim_matches('"').to_string();
                }
                "package_identifier" => {
                    // 导入别名
                    alias = Some(source_code[child.byte_range()].to_string());
                }
                _ => {}
            }
        }

        if !path.is_empty() {
            Some(crate::parser::Import { path, alias })
        } else {
            None
        }
    }

    /// 提取 Go 声明
    fn extract_go_declarations(
        &self,
        root: tree_sitter::Node,
        source_code: &str,
        file_path: &Path,
    ) -> Result<Vec<Box<dyn crate::parser::common::Declaration>>> {
        use crate::parser::GoDeclaration;

        let mut declarations: Vec<Box<dyn crate::parser::common::Declaration>> = Vec::new();

        // 提取函数声明
        let functions = self.navigator.find_function_declarations(root);
        for func_node in functions {
            if let Ok(func_info) = self.extract_function_info(func_node, source_code, file_path) {
                declarations.push(Box::new(GoDeclaration::Function(func_info)));
            }
        }

        // 提取方法声明
        let methods = self.navigator.find_method_declarations(root);
        for method_node in methods {
            if let Ok(method_info) = self.extract_function_info(method_node, source_code, file_path)
            {
                declarations.push(Box::new(GoDeclaration::Method(method_info)));
            }
        }

        // 提取类型声明
        let types = self.navigator.find_type_declarations(root);
        for type_node in types {
            if let Some(type_def) = self.extract_type_definition(type_node, source_code, file_path)
            {
                declarations.push(Box::new(GoDeclaration::Type(type_def)));
            }
        }

        // 提取常量声明
        let constants = self.navigator.find_const_declarations(root);
        for const_node in constants {
            let const_defs = self.extract_constant_definitions(const_node, source_code, file_path);
            for const_def in const_defs {
                declarations.push(Box::new(GoDeclaration::Constant(const_def)));
            }
        }

        // 提取变量声明
        let variables = self.navigator.find_var_declarations(root);
        for var_node in variables {
            let var_defs = self.extract_variable_definitions(var_node, source_code, file_path);
            for var_def in var_defs {
                declarations.push(Box::new(GoDeclaration::Variable(var_def)));
            }
        }

        Ok(declarations)
    }

    /// 提取类型定义
    fn extract_type_definition(
        &self,
        type_node: tree_sitter::Node,
        source_code: &str,
        file_path: &Path,
    ) -> Option<GoTypeDefinition> {
        use crate::parser::GoTypeKind;

        let mut type_name = String::new();
        let mut type_kind = GoTypeKind::Alias;

        let mut cursor = type_node.walk();
        for child in type_node.children(&mut cursor) {
            if child.kind() == "type_spec" {
                // 在 type_spec 中查找类型名称和种类
                let mut spec_cursor = child.walk();
                for spec_child in child.children(&mut spec_cursor) {
                    match spec_child.kind() {
                        "type_identifier" => {
                            if type_name.is_empty() {
                                type_name = source_code[spec_child.byte_range()].to_string();
                            }
                        }
                        "struct_type" => {
                            type_kind = GoTypeKind::Struct;
                        }
                        "interface_type" => {
                            type_kind = GoTypeKind::Interface;
                        }
                        _ => {}
                    }
                }
            }
        }

        if !type_name.is_empty() {
            Some(GoTypeDefinition {
                name: type_name,
                kind: type_kind,
                definition: source_code[type_node.byte_range()].to_string(),
                file_path: file_path.to_path_buf(),
                dependencies: Vec::new(), // TODO: 在后续任务中实现依赖提取
            })
        } else {
            None
        }
    }

    /// 提取常量定义
    fn extract_constant_definitions(
        &self,
        const_node: tree_sitter::Node,
        source_code: &str,
        file_path: &Path,
    ) -> Vec<crate::parser::GoConstantDefinition> {
        use crate::parser::{GoConstantDefinition, GoType};

        let mut constants = Vec::new();
        let (start_line, end_line) = self.navigator.get_node_line_range(const_node);

        let mut cursor = const_node.walk();
        for child in const_node.children(&mut cursor) {
            if child.kind() == "const_spec" {
                // 提取常量规范
                let mut name = String::new();
                let mut value = String::new();
                let mut const_type = None;

                let mut spec_cursor = child.walk();
                for spec_child in child.children(&mut spec_cursor) {
                    match spec_child.kind() {
                        "identifier" => {
                            if name.is_empty() {
                                name = source_code[spec_child.byte_range()].to_string();
                            }
                        }
                        "type_identifier" => {
                            const_type = Some(GoType {
                                name: source_code[spec_child.byte_range()].to_string(),
                                is_pointer: false,
                                is_slice: false,
                            });
                        }
                        _ => {
                            // 其他节点可能是值表达式
                            if !name.is_empty() && value.is_empty() {
                                value = source_code[spec_child.byte_range()].to_string();
                            }
                        }
                    }
                }

                if !name.is_empty() {
                    constants.push(GoConstantDefinition {
                        name,
                        value,
                        const_type,
                        start_line,
                        end_line,
                        file_path: file_path.to_path_buf(),
                    });
                }
            }
        }

        constants
    }

    /// 提取变量定义
    fn extract_variable_definitions(
        &self,
        var_node: tree_sitter::Node,
        source_code: &str,
        file_path: &Path,
    ) -> Vec<crate::parser::GoVariableDefinition> {
        use crate::parser::{GoType, GoVariableDefinition};

        let mut variables = Vec::new();
        let (start_line, end_line) = self.navigator.get_node_line_range(var_node);

        let mut cursor = var_node.walk();
        for child in var_node.children(&mut cursor) {
            if child.kind() == "var_spec" {
                // 提取变量规范
                let mut name = String::new();
                let mut var_type = None;
                let mut initial_value = None;

                let mut spec_cursor = child.walk();
                for spec_child in child.children(&mut spec_cursor) {
                    match spec_child.kind() {
                        "identifier" => {
                            if name.is_empty() {
                                name = source_code[spec_child.byte_range()].to_string();
                            }
                        }
                        "type_identifier" => {
                            var_type = Some(GoType {
                                name: source_code[spec_child.byte_range()].to_string(),
                                is_pointer: false,
                                is_slice: false,
                            });
                        }
                        _ => {
                            // 其他节点可能是初始值表达式
                            if !name.is_empty() && initial_value.is_none() {
                                initial_value =
                                    Some(source_code[spec_child.byte_range()].to_string());
                            }
                        }
                    }
                }

                if !name.is_empty() {
                    variables.push(GoVariableDefinition {
                        name,
                        var_type,
                        initial_value,
                        start_line,
                        end_line,
                        file_path: file_path.to_path_buf(),
                    });
                }
            }
        }

        variables
    }

    /// 获取分析器的语言类型
    pub fn language(&self) -> SupportedLanguage {
        self.language
    }

    /// 获取 CST 导航器的引用
    pub fn navigator(&self) -> &CstNavigator {
        &self.navigator
    }
}

impl Default for TypeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeAnalyzer {
    /// 创建新的类型分析器
    pub fn new() -> Self {
        // TODO: 在任务 8 中实现
        todo!("Implementation in task 8")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{DiffHunk, DiffLine, DiffLineType};
    use crate::parser::common::LanguageSpecificInfo;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    /// 创建测试用的临时 Go 文件
    fn create_test_go_file(content: &str) -> NamedTempFile {
        let mut temp_file = NamedTempFile::with_suffix(".go").expect("Failed to create temp file");
        std::io::Write::write_all(&mut temp_file, content.as_bytes())
            .expect("Failed to write to temp file");
        temp_file
    }

    #[test]
    fn test_source_analyzer_new_for_file() {
        // 测试根据文件路径创建分析器
        let go_file = PathBuf::from("test.go");
        let analyzer = SourceAnalyzer::new_for_file(&go_file);
        assert!(analyzer.is_ok(), "Should create analyzer for Go file");

        let analyzer = analyzer.unwrap();
        assert_eq!(analyzer.language(), SupportedLanguage::Go);

        // 测试不支持的文件类型
        let unknown_file = PathBuf::from("test.txt");
        let analyzer = SourceAnalyzer::new_for_file(&unknown_file);
        assert!(analyzer.is_err(), "Should fail for unsupported file type");
    }

    #[test]
    fn test_source_analyzer_new_for_language() {
        // 测试根据语言类型创建分析器
        let analyzer = SourceAnalyzer::new_for_language(SupportedLanguage::Go);
        assert!(analyzer.is_ok(), "Should create analyzer for Go language");

        let analyzer = analyzer.unwrap();
        assert_eq!(analyzer.language(), SupportedLanguage::Go);
    }

    #[test]
    fn test_analyze_simple_go_file() {
        let go_source = r#"package main

import "fmt"

func main() {
    fmt.Println("Hello, World!")
}

func add(a, b int) int {
    return a + b
}
"#;

        let temp_file = create_test_go_file(go_source);
        let file_path = temp_file.path();

        let mut analyzer =
            SourceAnalyzer::new_for_file(file_path).expect("Failed to create analyzer");

        let source_file = analyzer
            .analyze_file(file_path)
            .expect("Failed to analyze file");

        // 验证基本信息
        assert_eq!(source_file.path, file_path);
        assert_eq!(source_file.language, SupportedLanguage::Go);
        assert_eq!(source_file.source_code, go_source);

        // 验证语言特定信息
        let go_info = source_file
            .language_specific
            .as_any()
            .downcast_ref::<crate::parser::GoLanguageInfo>()
            .expect("Should be GoLanguageInfo");

        assert_eq!(go_info.package_name(), "main");
        assert_eq!(go_info.imports().len(), 1);
        assert_eq!(go_info.imports()[0].path, "fmt");
        assert!(go_info.imports()[0].alias.is_none());

        // 验证声明数量
        assert_eq!(go_info.declarations().len(), 2); // main 和 add 函数
    }

    #[test]
    fn test_analyze_go_file_with_types() {
        let go_source = r#"package main

import (
    "fmt"
    "strings"
)

const MaxSize = 100

var GlobalVar string = "global"

type Person struct {
    Name string
    Age  int
}

type Handler interface {
    Handle() error
}

func NewPerson(name string, age int) *Person {
    return &Person{Name: name, Age: age}
}

func (p *Person) Greet() string {
    return fmt.Sprintf("Hello, I'm %s", p.Name)
}
"#;

        let temp_file = create_test_go_file(go_source);
        let file_path = temp_file.path();

        let mut analyzer =
            SourceAnalyzer::new_for_file(file_path).expect("Failed to create analyzer");

        let source_file = analyzer
            .analyze_file(file_path)
            .expect("Failed to analyze file");

        let go_info = source_file
            .language_specific
            .as_any()
            .downcast_ref::<crate::parser::GoLanguageInfo>()
            .expect("Should be GoLanguageInfo");

        // 验证包名
        assert_eq!(go_info.package_name(), "main");

        // 验证导入
        assert_eq!(go_info.imports().len(), 2);
        let import_paths: Vec<&str> = go_info.imports().iter().map(|i| i.path.as_str()).collect();
        assert!(import_paths.contains(&"fmt"));
        assert!(import_paths.contains(&"strings"));

        // 验证声明数量（常量、变量、类型、函数、方法）
        assert!(go_info.declarations().len() >= 5);

        // 验证可以找到特定的函数和类型
        assert!(go_info.find_function("NewPerson").is_some());
        assert!(go_info.find_type("Person").is_some());
        assert!(go_info.find_type("Handler").is_some());
    }

    #[test]
    fn test_find_changed_functions() {
        let go_source = r#"package main

func main() {
    x := 42
    y := 24
    println(x + y)
}

func add(a, b int) int {
    return a + b
}

func multiply(x, y int) int {
    result := x * y
    return result
}
"#;

        let temp_file = create_test_go_file(go_source);
        let file_path = temp_file.path();

        let mut analyzer =
            SourceAnalyzer::new_for_file(file_path).expect("Failed to create analyzer");

        let source_file = analyzer
            .analyze_file(file_path)
            .expect("Failed to analyze file");

        // 创建模拟的差异块，表示第 4-5 行有变更（在 main 函数中）
        let hunks = vec![DiffHunk {
            old_start: 4,
            old_lines: 2,
            new_start: 4,
            new_lines: 2,
            lines: vec![
                DiffLine {
                    content: "    x := 42".to_string(),
                    line_type: DiffLineType::Context,
                },
                DiffLine {
                    content: "    y := 24".to_string(),
                    line_type: DiffLineType::Added,
                },
            ],
        }];

        let changed_functions = analyzer
            .find_changed_functions(&source_file, &hunks)
            .expect("Failed to find changed functions");

        // 应该找到 main 函数
        assert_eq!(changed_functions.len(), 1);
        assert_eq!(changed_functions[0].name, "main");

        // 创建另一个差异块，表示第 13-14 行有变更（在 multiply 函数中）
        let hunks2 = vec![DiffHunk {
            old_start: 13,
            old_lines: 2,
            new_start: 13,
            new_lines: 2,
            lines: vec![
                DiffLine {
                    content: "    result := x * y".to_string(),
                    line_type: DiffLineType::Added,
                },
                DiffLine {
                    content: "    return result".to_string(),
                    line_type: DiffLineType::Context,
                },
            ],
        }];

        let changed_functions2 = analyzer
            .find_changed_functions(&source_file, &hunks2)
            .expect("Failed to find changed functions");

        // 应该找到 multiply 函数
        assert_eq!(changed_functions2.len(), 1);
        assert_eq!(changed_functions2[0].name, "multiply");
    }

    #[test]
    fn test_find_changed_functions_multiple_hunks() {
        let go_source = r#"package main

func main() {
    x := 42
    println(x)
}

func helper() {
    fmt.Println("helper")
}

func add(a, b int) int {
    return a + b
}
"#;

        let temp_file = create_test_go_file(go_source);
        let file_path = temp_file.path();

        let mut analyzer =
            SourceAnalyzer::new_for_file(file_path).expect("Failed to create analyzer");

        let source_file = analyzer
            .analyze_file(file_path)
            .expect("Failed to analyze file");

        // 创建多个差异块，涉及不同的函数
        let hunks = vec![
            DiffHunk {
                old_start: 4,
                old_lines: 1,
                new_start: 4,
                new_lines: 1,
                lines: vec![DiffLine {
                    content: "    x := 42".to_string(),
                    line_type: DiffLineType::Added,
                }],
            },
            DiffHunk {
                old_start: 12,
                old_lines: 1,
                new_start: 12,
                new_lines: 1,
                lines: vec![DiffLine {
                    content: "    return a + b".to_string(),
                    line_type: DiffLineType::Added,
                }],
            },
        ];

        let changed_functions = analyzer
            .find_changed_functions(&source_file, &hunks)
            .expect("Failed to find changed functions");

        // 应该找到 main 和 add 函数
        assert_eq!(changed_functions.len(), 2);
        let function_names: Vec<&str> = changed_functions.iter().map(|f| f.name.as_str()).collect();
        assert!(function_names.contains(&"main"));
        assert!(function_names.contains(&"add"));
    }

    #[test]
    fn test_find_changed_functions_no_duplicates() {
        let go_source = r#"package main

func main() {
    x := 42
    y := 24
    z := x + y
    println(z)
}
"#;

        let temp_file = create_test_go_file(go_source);
        let file_path = temp_file.path();

        let mut analyzer =
            SourceAnalyzer::new_for_file(file_path).expect("Failed to create analyzer");

        let source_file = analyzer
            .analyze_file(file_path)
            .expect("Failed to analyze file");

        // 创建多个差异块，都在同一个函数中
        let hunks = vec![
            DiffHunk {
                old_start: 4,
                old_lines: 1,
                new_start: 4,
                new_lines: 1,
                lines: vec![DiffLine {
                    content: "    x := 42".to_string(),
                    line_type: DiffLineType::Added,
                }],
            },
            DiffHunk {
                old_start: 6,
                old_lines: 1,
                new_start: 6,
                new_lines: 1,
                lines: vec![DiffLine {
                    content: "    z := x + y".to_string(),
                    line_type: DiffLineType::Added,
                }],
            },
        ];

        let changed_functions = analyzer
            .find_changed_functions(&source_file, &hunks)
            .expect("Failed to find changed functions");

        // 应该只找到一个 main 函数，不重复
        assert_eq!(changed_functions.len(), 1);
        assert_eq!(changed_functions[0].name, "main");
    }

    #[test]
    fn test_extract_package_name() {
        let go_source = r#"package mypackage

func main() {}
"#;

        let temp_file = create_test_go_file(go_source);
        let file_path = temp_file.path();

        let mut analyzer =
            SourceAnalyzer::new_for_file(file_path).expect("Failed to create analyzer");

        let source_file = analyzer
            .analyze_file(file_path)
            .expect("Failed to analyze file");

        let go_info = source_file
            .language_specific
            .as_any()
            .downcast_ref::<crate::parser::GoLanguageInfo>()
            .expect("Should be GoLanguageInfo");

        assert_eq!(go_info.package_name(), "mypackage");
    }

    #[test]
    fn test_extract_imports() {
        let go_source = r#"package main

import "fmt"
import "os"

import (
    "strings"
    "net/http"
    alias "github.com/example/pkg"
)
"#;

        let temp_file = create_test_go_file(go_source);
        let file_path = temp_file.path();

        let mut analyzer =
            SourceAnalyzer::new_for_file(file_path).expect("Failed to create analyzer");

        let source_file = analyzer
            .analyze_file(file_path)
            .expect("Failed to analyze file");

        let go_info = source_file
            .language_specific
            .as_any()
            .downcast_ref::<crate::parser::GoLanguageInfo>()
            .expect("Should be GoLanguageInfo");

        let imports = go_info.imports();
        assert!(imports.len() >= 4); // 至少应该有 4 个导入

        let import_paths: Vec<&str> = imports.iter().map(|i| i.path.as_str()).collect();
        assert!(import_paths.contains(&"fmt"));
        assert!(import_paths.contains(&"os"));
        assert!(import_paths.contains(&"strings"));
        assert!(import_paths.contains(&"net/http"));

        // 检查别名导入
        let alias_import = imports.iter().find(|i| i.path == "github.com/example/pkg");
        if let Some(import) = alias_import {
            assert_eq!(import.alias, Some("alias".to_string()));
        }
    }

    #[test]
    fn test_extract_function_info() {
        let go_source = r#"package main

func add(a int, b int) (int, error) {
    if a < 0 || b < 0 {
        return 0, fmt.Errorf("negative numbers not allowed")
    }
    return a + b, nil
}

func (s *Server) Start(port int) error {
    return nil
}
"#;

        let temp_file = create_test_go_file(go_source);
        let file_path = temp_file.path();

        let mut analyzer =
            SourceAnalyzer::new_for_file(file_path).expect("Failed to create analyzer");

        let source_file = analyzer
            .analyze_file(file_path)
            .expect("Failed to analyze file");

        let go_info = source_file
            .language_specific
            .as_any()
            .downcast_ref::<crate::parser::GoLanguageInfo>()
            .expect("Should be GoLanguageInfo");

        // 查找 add 函数
        let add_func = go_info.find_function("add");
        assert!(add_func.is_some(), "Should find add function");

        let add_func = add_func.unwrap();
        assert_eq!(add_func.name, "add");
        assert!(add_func.receiver.is_none());
        assert!(add_func.start_line > 0);
        assert!(add_func.end_line > add_func.start_line);

        // 验证声明中包含函数和方法
        let declarations = go_info.declarations();
        let func_count = declarations
            .iter()
            .filter(|d| d.declaration_type() == "function")
            .count();
        let method_count = declarations
            .iter()
            .filter(|d| d.declaration_type() == "method")
            .count();

        assert_eq!(func_count, 1); // add 函数
        assert_eq!(method_count, 1); // Start 方法
    }

    #[test]
    fn test_analyzer_navigator_access() {
        let analyzer = SourceAnalyzer::new_for_language(SupportedLanguage::Go)
            .expect("Failed to create analyzer");

        // 测试可以访问导航器
        let _navigator = analyzer.navigator();
        // 注意：无法在没有实际语法树的情况下测试导航器的具体功能
    }

    #[test]
    fn test_find_function_containing_line_edge_cases() {
        let go_source = r#"package main

// 这是一个注释
func main() {
    // 函数内的注释
    x := 42
}

// 另一个注释
"#;

        let temp_file = create_test_go_file(go_source);
        let file_path = temp_file.path();

        let mut analyzer =
            SourceAnalyzer::new_for_file(file_path).expect("Failed to create analyzer");

        let source_file = analyzer
            .analyze_file(file_path)
            .expect("Failed to analyze file");

        // 测试在函数外的行
        let result = analyzer.find_function_containing_line(&source_file, 2);
        assert!(result.is_ok());
        assert!(
            result.unwrap().is_none(),
            "Line 2 should not be in any function"
        );

        // 测试在函数内的行
        let result = analyzer.find_function_containing_line(&source_file, 6);
        assert!(result.is_ok());
        let func_info = result.unwrap();
        assert!(func_info.is_some(), "Line 6 should be in main function");
        assert_eq!(func_info.unwrap().name, "main");

        // 测试超出文件范围的行
        let result = analyzer.find_function_containing_line(&source_file, 100);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none(), "Line 100 should not exist");
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::git::{DiffHunk, DiffLine, DiffLineType};
    use crate::parser::common::LanguageSpecificInfo;
    use std::fs;
    use tempfile::TempDir;

    /// 创建一个包含多个 Go 文件的临时目录
    fn create_test_project() -> TempDir {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // 创建 main.go
        let main_go = r#"package main

import (
    "fmt"
    "./utils"
)

func main() {
    result := utils.Add(10, 20)
    fmt.Printf("Result: %d\n", result)
}
"#;

        // 创建 utils/math.go
        let utils_dir = temp_dir.path().join("utils");
        fs::create_dir(&utils_dir).expect("Failed to create utils dir");

        let math_go = r#"package utils

func Add(a, b int) int {
    return a + b
}

func Multiply(a, b int) int {
    return a * b
}

type Calculator struct {
    name string
}

func (c *Calculator) Calculate(op string, a, b int) int {
    switch op {
    case "add":
        return Add(a, b)
    case "multiply":
        return Multiply(a, b)
    default:
        return 0
    }
}
"#;

        fs::write(temp_dir.path().join("main.go"), main_go).expect("Failed to write main.go");
        fs::write(utils_dir.join("math.go"), math_go).expect("Failed to write math.go");

        temp_dir
    }

    #[test]
    fn test_analyze_multiple_files() {
        let temp_dir = create_test_project();

        // 分析 main.go
        let main_path = temp_dir.path().join("main.go");
        let mut main_analyzer = SourceAnalyzer::new_for_file(&main_path)
            .expect("Failed to create analyzer for main.go");

        let main_source = main_analyzer
            .analyze_file(&main_path)
            .expect("Failed to analyze main.go");

        let main_go_info = main_source
            .language_specific
            .as_any()
            .downcast_ref::<crate::parser::GoLanguageInfo>()
            .expect("Should be GoLanguageInfo");

        assert_eq!(main_go_info.package_name(), "main");
        assert!(main_go_info.find_function("main").is_some());

        // 分析 utils/math.go
        let math_path = temp_dir.path().join("utils").join("math.go");
        let mut math_analyzer = SourceAnalyzer::new_for_file(&math_path)
            .expect("Failed to create analyzer for math.go");

        let math_source = math_analyzer
            .analyze_file(&math_path)
            .expect("Failed to analyze math.go");

        let math_go_info = math_source
            .language_specific
            .as_any()
            .downcast_ref::<crate::parser::GoLanguageInfo>()
            .expect("Should be GoLanguageInfo");

        assert_eq!(math_go_info.package_name(), "utils");
        assert!(math_go_info.find_function("Add").is_some());
        assert!(math_go_info.find_function("Multiply").is_some());
        assert!(math_go_info.find_type("Calculator").is_some());

        // 验证方法声明
        let declarations = math_go_info.declarations();
        let method_count = declarations
            .iter()
            .filter(|d| d.declaration_type() == "method")
            .count();
        assert_eq!(method_count, 1); // Calculate 方法
    }

    #[test]
    fn test_analyze_file_with_complex_changes() {
        let temp_dir = create_test_project();
        let math_path = temp_dir.path().join("utils").join("math.go");

        let mut analyzer =
            SourceAnalyzer::new_for_file(&math_path).expect("Failed to create analyzer");

        let source_file = analyzer
            .analyze_file(&math_path)
            .expect("Failed to analyze file");

        // 模拟复杂的变更：涉及多个函数的修改
        let hunks = vec![
            // Add 函数中的变更
            DiffHunk {
                old_start: 3,
                old_lines: 1,
                new_start: 3,
                new_lines: 2,
                lines: vec![
                    DiffLine {
                        content: "func Add(a, b int) int {".to_string(),
                        line_type: DiffLineType::Context,
                    },
                    DiffLine {
                        content: "    // Added validation".to_string(),
                        line_type: DiffLineType::Added,
                    },
                ],
            },
            // Calculator 方法中的变更
            DiffHunk {
                old_start: 17,
                old_lines: 2,
                new_start: 18,
                new_lines: 3,
                lines: vec![
                    DiffLine {
                        content: "func (c *Calculator) Calculate(op string, a, b int) int {"
                            .to_string(),
                        line_type: DiffLineType::Context,
                    },
                    DiffLine {
                        content: "    // Added logging".to_string(),
                        line_type: DiffLineType::Added,
                    },
                    DiffLine {
                        content: "    switch op {".to_string(),
                        line_type: DiffLineType::Context,
                    },
                ],
            },
        ];

        let changed_functions = analyzer
            .find_changed_functions(&source_file, &hunks)
            .expect("Failed to find changed functions");

        // 应该找到 Add 函数和 Calculate 方法
        assert_eq!(changed_functions.len(), 2);
        let function_names: Vec<&str> = changed_functions.iter().map(|f| f.name.as_str()).collect();
        assert!(function_names.contains(&"Add"));
        assert!(function_names.contains(&"Calculate"));
    }
}
