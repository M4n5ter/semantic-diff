//! 代码分析模块
//!
//! 提供依赖关系解析和语义分析功能

use crate::error::{Result, SemanticDiffError};
use crate::git::DiffHunk;
use crate::parser::{
    GoFunctionInfo, GoTypeDefinition, LanguageParser, ParserFactory, SourceFile, SupportedLanguage,
    common::{CstNavigator, LanguageSpecificInfo},
};
use std::collections::HashSet;
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
pub struct DependencyResolver {
    /// 项目的模块路径（从go.mod中获取）
    project_module_path: Option<String>,
}

/// 类型分析器
pub struct TypeAnalyzer;

/// 依赖关系信息
#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub dependency_type: DependencyType,
    pub file_path: std::path::PathBuf,
}

/// 依赖类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyType {
    Function,
    Type,
    Constant,
    Variable,
}

/// 类型引用
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeReference {
    pub name: String,
    pub package: Option<String>,
}

/// 函数调用
#[derive(Debug, Clone, PartialEq, Eq)]
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
        Self {
            project_module_path: None,
        }
    }

    /// 创建带有项目模块路径的依赖解析器
    pub fn new_with_project_path(project_module_path: String) -> Self {
        Self {
            project_module_path: Some(project_module_path),
        }
    }

    /// 从项目根目录的go.mod文件中读取模块路径
    pub fn from_project_root<P: AsRef<std::path::Path>>(project_root: P) -> Result<Self> {
        let go_mod_path = project_root.as_ref().join("go.mod");

        if go_mod_path.exists() {
            let content =
                std::fs::read_to_string(&go_mod_path).map_err(SemanticDiffError::IoError)?;

            if let Some(module_path) = Self::extract_module_path_from_go_mod(&content) {
                Ok(Self::new_with_project_path(module_path))
            } else {
                Ok(Self::new())
            }
        } else {
            Ok(Self::new())
        }
    }

    /// 从go.mod内容中提取模块路径
    fn extract_module_path_from_go_mod(content: &str) -> Option<String> {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("module ") {
                let module_path = line.strip_prefix("module ")?.trim();
                return Some(module_path.to_string());
            }
        }
        None
    }

    /// 解析类型引用，查找项目内部类型定义
    ///
    /// 在提供的源文件列表中查找指定类型的定义
    pub fn resolve_type(
        &self,
        type_ref: &TypeReference,
        source_files: &[SourceFile],
    ) -> Option<GoTypeDefinition> {
        // 遍历所有源文件查找类型定义
        for source_file in source_files {
            if let Some(type_def) = self.find_type_in_file(type_ref, source_file) {
                return Some(type_def);
            }
        }
        None
    }

    /// 解析函数调用，查找项目内部函数定义
    ///
    /// 在提供的源文件列表中查找指定函数的定义
    pub fn resolve_function(
        &self,
        func_call: &FunctionCall,
        source_files: &[SourceFile],
    ) -> Option<GoFunctionInfo> {
        // 遍历所有源文件查找函数定义
        for source_file in source_files {
            if let Some(func_info) = self.find_function_in_file(func_call, source_file) {
                return Some(func_info);
            }
        }
        None
    }

    /// 检查导入是否为外部依赖
    ///
    /// 区分项目内部代码和第三方库代码
    pub fn is_external_dependency(&self, import: &crate::parser::Import) -> bool {
        // 首先检查是否为项目内部包
        if self.is_project_internal_package(&import.path) {
            return false;
        }

        // 然后检查是否为标准库
        if self.is_standard_library(&import.path) {
            return true;
        }

        // 检查是否为第三方库
        if self.is_third_party_library(&import.path) {
            return true;
        }

        // 对于无法确定的包，如果没有项目模块路径信息，采用保守策略
        // 将其视为外部依赖以避免误判
        if self.project_module_path.is_none() {
            // 只有明确的相对路径才认为是内部的
            !import.path.starts_with("./") && !import.path.starts_with("../")
        } else {
            // 有项目模块路径时，不以项目路径开头的都是外部的
            true
        }
    }

    /// 在指定的源文件列表中查找类型定义
    ///
    /// 根据类型名称查找对应的类型定义
    pub fn find_type_definition(
        &self,
        type_name: &str,
        source_files: &[SourceFile],
    ) -> Option<GoTypeDefinition> {
        let type_ref = TypeReference {
            name: type_name.to_string(),
            package: None,
        };
        self.resolve_type(&type_ref, source_files)
    }

    /// 查找函数定义
    ///
    /// 根据函数名称查找对应的函数定义
    pub fn find_function_definition(
        &self,
        func_name: &str,
        source_files: &[SourceFile],
    ) -> Option<GoFunctionInfo> {
        let func_call = FunctionCall {
            name: func_name.to_string(),
            receiver: None,
            package: None,
        };
        self.resolve_function(&func_call, source_files)
    }

    /// 提取函数中的所有依赖
    ///
    /// 分析函数体，提取其中使用的类型和函数调用
    pub fn extract_function_dependencies(
        &self,
        function: &GoFunctionInfo,
        source_files: &[SourceFile],
    ) -> Vec<Dependency> {
        let mut dependencies = Vec::new();

        // 从函数参数中提取类型引用
        for param in &function.parameters {
            let type_refs = self.parse_type_string(&param.param_type.name);
            for type_ref in type_refs {
                if let Some(type_def) = self.resolve_type(&type_ref, source_files) {
                    dependencies.push(Dependency {
                        name: type_def.name.clone(),
                        dependency_type: DependencyType::Type,
                        file_path: type_def.file_path.clone(),
                    });
                }
            }
        }

        // 从返回值类型中提取类型引用
        for return_type in &function.return_types {
            let type_refs = self.parse_type_string(&return_type.name);
            for type_ref in type_refs {
                if let Some(type_def) = self.resolve_type(&type_ref, source_files) {
                    dependencies.push(Dependency {
                        name: type_def.name.clone(),
                        dependency_type: DependencyType::Type,
                        file_path: type_def.file_path.clone(),
                    });
                }
            }
        }

        // 从函数体中提取类型引用
        let type_refs = self.extract_type_references_from_code(&function.body);
        for type_ref in type_refs {
            if let Some(type_def) = self.resolve_type(&type_ref, source_files) {
                dependencies.push(Dependency {
                    name: type_def.name.clone(),
                    dependency_type: DependencyType::Type,
                    file_path: type_def.file_path.clone(),
                });
            }
        }

        // 从函数体中提取函数调用
        let func_calls = self.extract_function_calls_from_code(&function.body);
        for func_call in func_calls {
            if let Some(func_info) = self.resolve_function(&func_call, source_files) {
                dependencies.push(Dependency {
                    name: func_info.name.clone(),
                    dependency_type: DependencyType::Function,
                    file_path: func_info.file_path.clone(),
                });
            }
        }

        // 去重
        dependencies.sort_by(|a, b| a.name.cmp(&b.name));
        dependencies.dedup_by(|a, b| a.name == b.name && a.dependency_type == b.dependency_type);

        dependencies
    }

    /// 过滤外部依赖
    ///
    /// 从依赖列表中移除第三方库的依赖，只保留项目内部依赖
    pub fn filter_internal_dependencies(&self, dependencies: &[Dependency]) -> Vec<Dependency> {
        dependencies
            .iter()
            .filter(|dep| self.is_internal_dependency(dep))
            .cloned()
            .collect()
    }

    /// 在单个文件中查找类型定义
    fn find_type_in_file(
        &self,
        type_ref: &TypeReference,
        source_file: &SourceFile,
    ) -> Option<GoTypeDefinition> {
        // 获取 Go 语言特定信息
        let go_info = source_file
            .language_specific
            .as_any()
            .downcast_ref::<crate::parser::GoLanguageInfo>()?;

        // 在声明中查找匹配的类型
        for declaration in go_info.declarations() {
            if let Some(crate::parser::GoDeclaration::Type(type_def)) = declaration
                .as_any()
                .downcast_ref::<crate::parser::GoDeclaration>(
            ) {
                if self.type_matches(type_ref, type_def, go_info) {
                    return Some(type_def.clone());
                }
            }
        }
        None
    }

    /// 在单个文件中查找函数定义
    fn find_function_in_file(
        &self,
        func_call: &FunctionCall,
        source_file: &SourceFile,
    ) -> Option<GoFunctionInfo> {
        // 获取 Go 语言特定信息
        let go_info = source_file
            .language_specific
            .as_any()
            .downcast_ref::<crate::parser::GoLanguageInfo>()?;

        // 在声明中查找匹配的函数或方法
        for declaration in go_info.declarations() {
            if let Some(go_decl) = declaration
                .as_any()
                .downcast_ref::<crate::parser::GoDeclaration>()
            {
                match go_decl {
                    crate::parser::GoDeclaration::Function(func_info) => {
                        if self.function_matches(func_call, func_info, go_info) {
                            return Some(func_info.clone());
                        }
                    }
                    crate::parser::GoDeclaration::Method(method_info) => {
                        if self.method_matches(func_call, method_info, go_info) {
                            return Some(method_info.clone());
                        }
                    }
                    _ => {}
                }
            }
        }

        None
    }

    /// 检查类型是否匹配
    fn type_matches(
        &self,
        type_ref: &TypeReference,
        type_def: &GoTypeDefinition,
        go_info: &crate::parser::GoLanguageInfo,
    ) -> bool {
        // 简单名称匹配
        if type_ref.name == type_def.name {
            // 如果没有指定包名，或者包名匹配
            if type_ref.package.is_none()
                || type_ref.package.as_deref() == Some(go_info.package_name())
            {
                return true;
            }
        }

        // 处理包限定的类型名称
        if let Some(package) = &type_ref.package {
            // 检查是否有对应的导入别名
            for import in go_info.imports() {
                if let Some(alias) = &import.alias {
                    if alias == package && type_ref.name == type_def.name {
                        return true;
                    }
                } else {
                    // 使用包路径的最后一部分作为包名
                    let package_name = import.path.split('/').next_back().unwrap_or(&import.path);
                    if package_name == package && type_ref.name == type_def.name {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// 检查函数是否匹配
    fn function_matches(
        &self,
        func_call: &FunctionCall,
        func_info: &GoFunctionInfo,
        go_info: &crate::parser::GoLanguageInfo,
    ) -> bool {
        // 简单名称匹配
        if func_call.name == func_info.name {
            // 如果没有指定包名，或者包名匹配
            if func_call.package.is_none()
                || func_call.package.as_deref() == Some(go_info.package_name())
            {
                return true;
            }
        }

        // 处理包限定的函数名称
        if let Some(package) = &func_call.package {
            for import in go_info.imports() {
                if let Some(alias) = &import.alias {
                    if alias == package && func_call.name == func_info.name {
                        return true;
                    }
                } else {
                    let package_name = import.path.split('/').next_back().unwrap_or(&import.path);
                    if package_name == package && func_call.name == func_info.name {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// 检查方法是否匹配
    fn method_matches(
        &self,
        func_call: &FunctionCall,
        method_info: &GoFunctionInfo,
        _go_info: &crate::parser::GoLanguageInfo,
    ) -> bool {
        // 方法匹配需要考虑接收者
        if func_call.name == method_info.name {
            // 如果指定了接收者，检查接收者类型是否匹配
            if let Some(receiver_name) = &func_call.receiver {
                if let Some(receiver_info) = &method_info.receiver {
                    return receiver_name == &receiver_info.type_name;
                }
            } else {
                // 如果没有指定接收者，也可能匹配（在同一个包内调用）
                return true;
            }
        }

        false
    }

    /// 检查是否为标准库
    fn is_standard_library(&self, import_path: &str) -> bool {
        // Go 标准库的常见包
        const STANDARD_PACKAGES: &[&str] = &[
            "fmt",
            "os",
            "io",
            "net",
            "http",
            "strings",
            "strconv",
            "time",
            "context",
            "encoding/json",
            "net/http",
            "database/sql",
            "crypto",
            "math",
            "sort",
            "bufio",
            "bytes",
            "errors",
            "flag",
            "log",
            "path",
            "regexp",
            "sync",
            "testing",
            "unsafe",
            "reflect",
            "runtime",
            "syscall",
        ];

        // 检查是否为标准库包
        STANDARD_PACKAGES.contains(&import_path) ||
        // 或者是标准库的子包
        STANDARD_PACKAGES.iter().any(|&std_pkg| import_path.starts_with(&format!("{std_pkg}/")))
    }

    /// 检查是否为项目内部包
    fn is_project_internal_package(&self, import_path: &str) -> bool {
        if let Some(project_module_path) = &self.project_module_path {
            // 如果导入路径以项目模块路径开头，则认为是项目内部包
            import_path.starts_with(project_module_path)
        } else {
            // 如果没有项目模块路径信息，使用保守的启发式方法
            // 只有明确的相对路径导入才认为是项目内部的
            import_path.starts_with("./") || import_path.starts_with("../")
        }
    }

    /// 检查是否为第三方库
    fn is_third_party_library(&self, import_path: &str) -> bool {
        // 第三方库通常包含域名
        import_path.starts_with("github.com/")
            || import_path.starts_with("gitlab.com/")
            || import_path.starts_with("bitbucket.org/")
            || import_path.starts_with("golang.org/")
            || import_path.starts_with("google.golang.org/")
            || import_path.starts_with("go.uber.org/")
            || (import_path.contains('.')
                && (import_path.contains(".com/")
                    || import_path.contains(".org/")
                    || import_path.contains(".net/")))
    }

    /// 检查是否为项目内部依赖
    fn is_internal_dependency(&self, dependency: &Dependency) -> bool {
        // 简单的启发式方法：如果文件路径不包含 vendor 或者第三方库的特征，认为是内部依赖
        let path_str = dependency.file_path.to_string_lossy();
        !path_str.contains("vendor/")
            && !path_str.contains("go/pkg/mod/")
            && !path_str.contains(".cache/")
    }

    /// 从代码中提取类型引用
    pub fn extract_type_references_from_code(&self, code: &str) -> Vec<TypeReference> {
        let mut type_refs = Vec::new();

        // 简单的正则表达式匹配（实际实现中应该使用语法树分析）
        // 这里提供一个基础实现，实际应该通过 tree-sitter 分析

        // 匹配类型声明模式，如 var x Type, func() Type, *Type 等
        let patterns = [
            r"\b([A-Z][a-zA-Z0-9_]*)\s*\{",        // 结构体字面量
            r"\*([A-Z][a-zA-Z0-9_]*)\b",           // 指针类型
            r"\[\]([A-Z][a-zA-Z0-9_]*)\b",         // 切片类型
            r"var\s+\w+\s+([A-Z][a-zA-Z0-9_]*)\b", // 变量声明
            r":\s*([A-Z][a-zA-Z0-9_]*)\b",         // 短变量声明
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for cap in re.captures_iter(code) {
                    if let Some(type_name) = cap.get(1) {
                        let type_ref = TypeReference {
                            name: type_name.as_str().to_string(),
                            package: None,
                        };
                        if !type_refs
                            .iter()
                            .any(|t: &TypeReference| t.name == type_ref.name)
                        {
                            type_refs.push(type_ref);
                        }
                    }
                }
            }
        }

        type_refs
    }

    /// 从代码中提取函数调用
    fn extract_function_calls_from_code(&self, code: &str) -> Vec<FunctionCall> {
        let mut func_calls = Vec::new();

        // 简单的正则表达式匹配函数调用
        let patterns = [
            r"([a-zA-Z_][a-zA-Z0-9_]*)\s*\(", // 简单函数调用
            r"([a-zA-Z_][a-zA-Z0-9_]*)\.([a-zA-Z_][a-zA-Z0-9_]*)\s*\(", // 方法调用
        ];

        // 匹配简单函数调用
        if let Ok(re) = regex::Regex::new(patterns[0]) {
            for cap in re.captures_iter(code) {
                if let Some(func_name) = cap.get(1) {
                    let func_call = FunctionCall {
                        name: func_name.as_str().to_string(),
                        receiver: None,
                        package: None,
                    };
                    if !func_calls
                        .iter()
                        .any(|f: &FunctionCall| f.name == func_call.name)
                    {
                        func_calls.push(func_call);
                    }
                }
            }
        }

        // 匹配方法调用
        if let Ok(re) = regex::Regex::new(patterns[1]) {
            for cap in re.captures_iter(code) {
                if let (Some(receiver), Some(method)) = (cap.get(1), cap.get(2)) {
                    let func_call = FunctionCall {
                        name: method.as_str().to_string(),
                        receiver: Some(receiver.as_str().to_string()),
                        package: None,
                    };
                    if !func_calls.iter().any(|f: &FunctionCall| {
                        f.name == func_call.name && f.receiver == func_call.receiver
                    }) {
                        func_calls.push(func_call);
                    }
                }
            }
        }

        func_calls
    }

    /// 解析类型字符串，提取其中的类型引用
    fn parse_type_string(&self, type_str: &str) -> Vec<TypeReference> {
        let mut type_refs = Vec::new();

        // 清理类型字符串，移除指针、切片等修饰符
        let clean_type = type_str
            .trim()
            .trim_start_matches("user ") // 移除变量名
            .trim_start_matches('*') // 移除指针标记
            .trim_start_matches("[]") // 移除切片标记
            .trim();

        // 检查是否包含包名
        if let Some(dot_pos) = clean_type.find('.') {
            let package = clean_type[..dot_pos].to_string();
            let type_name = clean_type[dot_pos + 1..].to_string();

            // 只有当类型名以大写字母开头时才认为是用户定义的类型
            if type_name.chars().next().is_some_and(|c| c.is_uppercase()) {
                type_refs.push(TypeReference {
                    name: type_name,
                    package: Some(package),
                });
            }
        } else if clean_type.chars().next().is_some_and(|c| c.is_uppercase()) {
            // 没有包名的类型
            type_refs.push(TypeReference {
                name: clean_type.to_string(),
                package: None,
            });
        }

        type_refs
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
        Self
    }

    /// 分析类型的依赖关系
    ///
    /// 提取类型定义中引用的其他类型
    pub fn analyze_type_dependencies(
        &self,
        type_def: &GoTypeDefinition,
        source_files: &[SourceFile],
    ) -> Vec<Dependency> {
        self.analyze_type_dependencies_recursive(type_def, source_files, &mut HashSet::new())
    }

    /// 递归分析类型的依赖关系，包括间接依赖
    ///
    /// 使用visited集合避免循环依赖导致的无限递归
    fn analyze_type_dependencies_recursive(
        &self,
        type_def: &GoTypeDefinition,
        source_files: &[SourceFile],
        visited: &mut HashSet<String>,
    ) -> Vec<Dependency> {
        let mut dependencies = Vec::new();

        // 避免循环依赖
        if visited.contains(&type_def.name) {
            return dependencies;
        }
        visited.insert(type_def.name.clone());

        // 从类型定义中提取类型引用
        let type_refs = self.extract_type_references_from_definition(&type_def.definition);

        for type_ref in type_refs {
            // 查找类型定义
            let resolver = DependencyResolver::new();
            if let Some(dep_type_def) = resolver.resolve_type(&type_ref, source_files) {
                // 添加直接依赖
                dependencies.push(Dependency {
                    name: dep_type_def.name.clone(),
                    dependency_type: DependencyType::Type,
                    file_path: dep_type_def.file_path.clone(),
                });

                // 递归分析间接依赖
                let indirect_deps =
                    self.analyze_type_dependencies_recursive(&dep_type_def, source_files, visited);
                for indirect_dep in indirect_deps {
                    // 避免重复添加
                    if !dependencies.iter().any(|d| d.name == indirect_dep.name) {
                        dependencies.push(indirect_dep);
                    }
                }
            }
        }

        // 从visited集合中移除当前类型，允许在其他路径中再次访问
        visited.remove(&type_def.name);

        dependencies
    }

    /// 分析结构体字段的类型
    ///
    /// 提取结构体中所有字段的类型信息
    pub fn analyze_struct_fields(&self, struct_def: &str) -> Vec<TypeReference> {
        let mut field_types = Vec::new();

        // 简单的字段类型提取（实际应该使用语法树分析）
        let lines: Vec<&str> = struct_def.lines().collect();
        let mut in_struct_body = false;

        for line in lines {
            let trimmed = line.trim();

            if trimmed.contains("struct {") {
                in_struct_body = true;
                continue;
            }

            if trimmed == "}" && in_struct_body {
                break;
            }

            if in_struct_body && !trimmed.is_empty() && !trimmed.starts_with("//") {
                // 解析字段行，格式如: FieldName Type `tag`
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    let field_type = parts[1];
                    let type_ref = self.parse_type_reference(field_type);

                    // 过滤掉内置类型和已存在的类型
                    if !self.is_builtin_type(&type_ref.name)
                        && !field_types
                            .iter()
                            .any(|t: &TypeReference| t.name == type_ref.name)
                    {
                        field_types.push(type_ref);
                    }
                }
            }
        }

        field_types
    }

    /// 分析接口方法的类型
    ///
    /// 提取接口中所有方法的参数和返回值类型
    pub fn analyze_interface_methods(&self, interface_def: &str) -> Vec<TypeReference> {
        let mut method_types = Vec::new();

        let lines: Vec<&str> = interface_def.lines().collect();
        let mut in_interface_body = false;

        for line in lines {
            let trimmed = line.trim();

            if trimmed.contains("interface {") {
                in_interface_body = true;
                continue;
            }

            if trimmed == "}" && in_interface_body {
                break;
            }

            if in_interface_body && !trimmed.is_empty() && !trimmed.starts_with("//") {
                // 解析方法签名中的类型
                let types = self.extract_types_from_method_signature(trimmed);
                for type_ref in types {
                    if !method_types
                        .iter()
                        .any(|t: &TypeReference| t.name == type_ref.name)
                    {
                        method_types.push(type_ref);
                    }
                }
            }
        }

        method_types
    }

    /// 从类型定义中提取类型引用
    fn extract_type_references_from_definition(&self, definition: &str) -> Vec<TypeReference> {
        let mut type_refs = Vec::new();

        // 根据类型定义的种类进行不同的处理
        if definition.contains("struct {") {
            type_refs.extend(self.analyze_struct_fields(definition));
        } else if definition.contains("interface {") {
            type_refs.extend(self.analyze_interface_methods(definition));
        } else {
            // 类型别名，直接提取右侧的类型
            if let Some(alias_type) = self.extract_alias_type(definition) {
                type_refs.push(alias_type);
            }
        }

        type_refs
    }

    /// 解析类型引用字符串
    fn parse_type_reference(&self, type_str: &str) -> TypeReference {
        // 处理各种类型格式
        let clean_type = type_str
            .trim_start_matches('*') // 移除指针标记
            .trim_start_matches("[]") // 移除切片标记
            .trim_start_matches("map[")
            .trim_end_matches(']');

        // 检查是否包含包名
        if let Some(dot_pos) = clean_type.find('.') {
            let package = clean_type[..dot_pos].to_string();
            let type_name = clean_type[dot_pos + 1..].to_string();
            TypeReference {
                name: type_name,
                package: Some(package),
            }
        } else {
            TypeReference {
                name: clean_type.to_string(),
                package: None,
            }
        }
    }

    /// 检查是否为Go内置类型
    fn is_builtin_type(&self, type_name: &str) -> bool {
        const BUILTIN_TYPES: &[&str] = &[
            "bool",
            "byte",
            "complex64",
            "complex128",
            "error",
            "float32",
            "float64",
            "int",
            "int8",
            "int16",
            "int32",
            "int64",
            "rune",
            "string",
            "uint",
            "uint8",
            "uint16",
            "uint32",
            "uint64",
            "uintptr",
        ];
        BUILTIN_TYPES.contains(&type_name)
    }

    /// 从方法签名中提取类型
    fn extract_types_from_method_signature(&self, signature: &str) -> Vec<TypeReference> {
        let mut types = Vec::new();

        // 简单的类型提取，匹配参数和返回值中的类型
        if let Ok(re) = regex::Regex::new(r"\b([A-Z][a-zA-Z0-9_]*)\b") {
            for cap in re.captures_iter(signature) {
                if let Some(type_match) = cap.get(1) {
                    let type_ref = self.parse_type_reference(type_match.as_str());
                    if !types
                        .iter()
                        .any(|t: &TypeReference| t.name == type_ref.name)
                    {
                        types.push(type_ref);
                    }
                }
            }
        }

        types
    }

    /// 提取类型别名的目标类型
    fn extract_alias_type(&self, definition: &str) -> Option<TypeReference> {
        // 匹配 type Alias = TargetType 的模式
        if let Ok(re) = regex::Regex::new(r"type\s+\w+\s*=\s*(.+)") {
            if let Some(cap) = re.captures(definition) {
                if let Some(target_type) = cap.get(1) {
                    return Some(self.parse_type_reference(target_type.as_str().trim()));
                }
            }
        }
        None
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
                    old_line_number: Some(4),
                    new_line_number: Some(4),
                },
                DiffLine {
                    content: "    y := 24".to_string(),
                    line_type: DiffLineType::Added,
                    old_line_number: None,
                    new_line_number: Some(5),
                },
            ],
            context_lines: 3,
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
                    old_line_number: None,
                    new_line_number: Some(13),
                },
                DiffLine {
                    content: "    return result".to_string(),
                    line_type: DiffLineType::Context,
                    old_line_number: Some(14),
                    new_line_number: Some(14),
                },
            ],
            context_lines: 3,
        }];

        let changed_functions2 = analyzer
            .find_changed_functions(&source_file, &hunks2)
            .expect("Failed to find changed functions");

        // 应该找到 multiply 函数
        assert_eq!(changed_functions2.len(), 1);
        assert_eq!(changed_functions2[0].name, "multiply");
    }

    #[test]
    fn test_dependency_resolver_creation() {
        let resolver = DependencyResolver::new();
        let _default_resolver = DependencyResolver::default();

        // 测试基本功能可用性
        let type_ref = TypeReference {
            name: "TestType".to_string(),
            package: None,
        };

        let func_call = FunctionCall {
            name: "testFunc".to_string(),
            receiver: None,
            package: None,
        };

        // 空的源文件列表应该返回 None
        assert!(resolver.resolve_type(&type_ref, &[]).is_none());
        assert!(resolver.resolve_function(&func_call, &[]).is_none());
    }

    #[test]
    fn test_is_external_dependency() {
        let resolver = DependencyResolver::new();

        // 测试标准库
        let std_import = crate::parser::Import {
            path: "fmt".to_string(),
            alias: None,
        };
        assert!(resolver.is_external_dependency(&std_import));

        let http_import = crate::parser::Import {
            path: "net/http".to_string(),
            alias: None,
        };
        assert!(resolver.is_external_dependency(&http_import));

        // 测试第三方库
        let github_import = crate::parser::Import {
            path: "github.com/user/repo".to_string(),
            alias: None,
        };
        assert!(resolver.is_external_dependency(&github_import));

        // 测试项目内部包（相对路径）
        let internal_import = crate::parser::Import {
            path: "./internal/utils".to_string(),
            alias: None,
        };
        assert!(!resolver.is_external_dependency(&internal_import));

        // 注意：没有项目模块路径时，非相对路径的包会被当作外部依赖处理
        let relative_import = crate::parser::Import {
            path: "myproject/internal".to_string(),
            alias: None,
        };
        assert!(resolver.is_external_dependency(&relative_import));
    }

    #[test]
    fn test_is_external_dependency_with_project_path() {
        // 测试带有项目模块路径的解析器
        let resolver =
            DependencyResolver::new_with_project_path("github.com/M4n5ter/examplePkg".to_string());

        // 测试项目内部包
        let project_internal = crate::parser::Import {
            path: "github.com/M4n5ter/examplePkg/internal/utils".to_string(),
            alias: None,
        };
        assert!(!resolver.is_external_dependency(&project_internal));

        let project_subpackage = crate::parser::Import {
            path: "github.com/M4n5ter/examplePkg/api/v1".to_string(),
            alias: None,
        };
        assert!(!resolver.is_external_dependency(&project_subpackage));

        // 测试第三方库（即使有相似的前缀）
        let third_party = crate::parser::Import {
            path: "github.com/M4n5ter/otherPkg".to_string(),
            alias: None,
        };
        assert!(resolver.is_external_dependency(&third_party));

        let another_third_party = crate::parser::Import {
            path: "github.com/other/repo".to_string(),
            alias: None,
        };
        assert!(resolver.is_external_dependency(&another_third_party));

        // 测试标准库
        let std_import = crate::parser::Import {
            path: "fmt".to_string(),
            alias: None,
        };
        assert!(resolver.is_external_dependency(&std_import));
    }

    #[test]
    fn test_extract_module_path_from_go_mod() {
        let go_mod_content = r#"module github.com/M4n5ter/examplePkg

go 1.21

require (
    github.com/stretchr/testify v1.8.4
)
"#;

        let module_path = DependencyResolver::extract_module_path_from_go_mod(go_mod_content);
        assert_eq!(
            module_path,
            Some("github.com/M4n5ter/examplePkg".to_string())
        );

        // 测试没有module声明的情况
        let invalid_content = r#"go 1.21

require (
    github.com/stretchr/testify v1.8.4
)
"#;
        let module_path = DependencyResolver::extract_module_path_from_go_mod(invalid_content);
        assert_eq!(module_path, None);

        // 测试带有注释的module声明
        let commented_content = r#"// This is a comment
module github.com/example/project

go 1.21
"#;
        let module_path = DependencyResolver::extract_module_path_from_go_mod(commented_content);
        assert_eq!(module_path, Some("github.com/example/project".to_string()));
    }

    #[test]
    fn test_is_project_internal_package() {
        // 测试带有项目路径的解析器
        let resolver =
            DependencyResolver::new_with_project_path("github.com/M4n5ter/examplePkg".to_string());

        assert!(resolver.is_project_internal_package("github.com/M4n5ter/examplePkg"));
        assert!(resolver.is_project_internal_package("github.com/M4n5ter/examplePkg/internal"));
        assert!(resolver.is_project_internal_package("github.com/M4n5ter/examplePkg/api/v1"));
        assert!(!resolver.is_project_internal_package("github.com/M4n5ter/otherPkg"));
        assert!(!resolver.is_project_internal_package("github.com/other/repo"));

        // 测试没有项目路径的解析器
        let resolver_no_path = DependencyResolver::new();
        assert!(resolver_no_path.is_project_internal_package("./internal"));
        assert!(resolver_no_path.is_project_internal_package("../utils"));
        assert!(!resolver_no_path.is_project_internal_package("localpackage")); // 保守策略：不确定的包当作外部处理
        assert!(!resolver_no_path.is_project_internal_package("github.com/user/repo"));
    }

    #[test]
    fn test_extract_type_references_from_code() {
        let resolver = DependencyResolver::new();

        let code = r#"
        var user User
        var users []User
        var userPtr *User
        config := Config{Port: 8080}
        var mapping map[string]Handler
        "#;

        let type_refs = resolver.extract_type_references_from_code(code);

        // 应该找到 User, Config, Handler 等类型
        assert!(!type_refs.is_empty());

        let type_names: Vec<&str> = type_refs.iter().map(|t| t.name.as_str()).collect();
        assert!(type_names.contains(&"User"));
        assert!(type_names.contains(&"Config"));
    }

    #[test]
    fn test_extract_function_calls_from_code() {
        let resolver = DependencyResolver::new();

        let code = r#"
        fmt.Println("hello")
        result := add(1, 2)
        server.Start()
        user.GetName()
        "#;

        let func_calls = resolver.extract_function_calls_from_code(code);

        // 应该找到函数调用
        assert!(!func_calls.is_empty());

        let func_names: Vec<&str> = func_calls.iter().map(|f| f.name.as_str()).collect();
        assert!(func_names.contains(&"Println") || func_names.contains(&"add"));
    }

    #[test]
    fn test_filter_internal_dependencies() {
        let resolver = DependencyResolver::new();

        let dependencies = vec![
            Dependency {
                name: "InternalType".to_string(),
                dependency_type: DependencyType::Type,
                file_path: PathBuf::from("internal/types.go"),
            },
            Dependency {
                name: "VendorType".to_string(),
                dependency_type: DependencyType::Type,
                file_path: PathBuf::from("vendor/github.com/user/repo/types.go"),
            },
            Dependency {
                name: "LocalFunc".to_string(),
                dependency_type: DependencyType::Function,
                file_path: PathBuf::from("utils/helper.go"),
            },
        ];

        let internal_deps = resolver.filter_internal_dependencies(&dependencies);

        // 应该过滤掉 vendor 目录下的依赖
        assert_eq!(internal_deps.len(), 2);
        assert!(
            internal_deps
                .iter()
                .all(|d| !d.file_path.to_string_lossy().contains("vendor"))
        );
    }

    #[test]
    fn test_type_analyzer_creation() {
        let analyzer = TypeAnalyzer::new();
        let _default_analyzer = TypeAnalyzer;

        // 测试基本功能
        let struct_def = r#"
        type User struct {
            Name    string
            Age     int
            Address *Address
        }
        "#;

        let field_types = analyzer.analyze_struct_fields(struct_def);
        assert!(!field_types.is_empty());

        let type_names: Vec<&str> = field_types.iter().map(|t| t.name.as_str()).collect();
        assert!(type_names.contains(&"Address"));
    }

    #[test]
    fn test_analyze_struct_fields() {
        let analyzer = TypeAnalyzer::new();

        let struct_def = r#"
        type Person struct {
            Name     string    `json:"name"`
            Age      int       `json:"age"`
            Address  *Address  `json:"address"`
            Tags     []string  `json:"tags"`
            Config   Config    `json:"config"`
        }
        "#;

        let field_types = analyzer.analyze_struct_fields(struct_def);

        // 应该找到 Address 和 Config 类型
        assert!(!field_types.is_empty());

        let type_names: Vec<&str> = field_types.iter().map(|t| t.name.as_str()).collect();
        assert!(type_names.contains(&"Address"));
        assert!(type_names.contains(&"Config"));

        // 基本类型不应该被包含
        assert!(!type_names.contains(&"string"));
        assert!(!type_names.contains(&"int"));
    }

    #[test]
    fn test_analyze_interface_methods() {
        let analyzer = TypeAnalyzer::new();

        let interface_def = r#"
        type Handler interface {
            Handle(request Request) Response
            Validate(data Data) error
            Process(input Input, output Output) Status
        }
        "#;

        let method_types = analyzer.analyze_interface_methods(interface_def);

        // 应该找到方法签名中的类型
        assert!(!method_types.is_empty());

        let type_names: Vec<&str> = method_types.iter().map(|t| t.name.as_str()).collect();
        assert!(type_names.contains(&"Request"));
        assert!(type_names.contains(&"Response"));
        assert!(type_names.contains(&"Data"));
    }

    #[test]
    fn test_parse_type_reference() {
        let analyzer = TypeAnalyzer::new();

        // 测试简单类型
        let simple_type = analyzer.parse_type_reference("User");
        assert_eq!(simple_type.name, "User");
        assert_eq!(simple_type.package, None);

        // 测试指针类型
        let pointer_type = analyzer.parse_type_reference("*User");
        assert_eq!(pointer_type.name, "User");
        assert_eq!(pointer_type.package, None);

        // 测试切片类型
        let slice_type = analyzer.parse_type_reference("[]User");
        assert_eq!(slice_type.name, "User");
        assert_eq!(slice_type.package, None);

        // 测试包限定类型
        let qualified_type = analyzer.parse_type_reference("http.Request");
        assert_eq!(qualified_type.name, "Request");
        assert_eq!(qualified_type.package, Some("http".to_string()));
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
                    old_line_number: None,
                    new_line_number: Some(4),
                }],
                context_lines: 3,
            },
            DiffHunk {
                old_start: 12,
                old_lines: 1,
                new_start: 12,
                new_lines: 1,
                lines: vec![DiffLine {
                    content: "    return a + b".to_string(),
                    line_type: DiffLineType::Added,
                    old_line_number: None,
                    new_line_number: Some(12),
                }],
                context_lines: 3,
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
                    old_line_number: None,
                    new_line_number: Some(4),
                }],
                context_lines: 3,
            },
            DiffHunk {
                old_start: 6,
                old_lines: 1,
                new_start: 6,
                new_lines: 1,
                lines: vec![DiffLine {
                    content: "    z := x + y".to_string(),
                    line_type: DiffLineType::Added,
                    old_line_number: None,
                    new_line_number: Some(6),
                }],
                context_lines: 3,
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
                        old_line_number: Some(3),
                        new_line_number: Some(3),
                    },
                    DiffLine {
                        content: "    // Added validation".to_string(),
                        line_type: DiffLineType::Added,
                        old_line_number: None,
                        new_line_number: Some(4),
                    },
                ],
                context_lines: 3,
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
                        old_line_number: Some(17),
                        new_line_number: Some(18),
                    },
                    DiffLine {
                        content: "    // Added logging".to_string(),
                        line_type: DiffLineType::Added,
                        old_line_number: None,
                        new_line_number: Some(19),
                    },
                    DiffLine {
                        content: "    switch op {".to_string(),
                        line_type: DiffLineType::Context,
                        old_line_number: Some(18),
                        new_line_number: Some(20),
                    },
                ],
                context_lines: 3,
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

    #[test]
    fn test_dependency_resolver_from_project_root() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // 创建 go.mod 文件
        let go_mod_content = r#"module github.com/M4n5ter/examplePkg

go 1.21

require (
    github.com/stretchr/testify v1.8.4
    github.com/other/library v1.0.0
)
"#;

        fs::write(temp_dir.path().join("go.mod"), go_mod_content).expect("Failed to write go.mod");

        // 从项目根目录创建解析器
        let resolver = DependencyResolver::from_project_root(temp_dir.path())
            .expect("Failed to create resolver from project root");

        // 测试项目内部包识别
        let internal_import = crate::parser::Import {
            path: "github.com/M4n5ter/examplePkg/internal/utils".to_string(),
            alias: None,
        };
        assert!(!resolver.is_external_dependency(&internal_import));

        // 测试第三方库识别
        let external_import = crate::parser::Import {
            path: "github.com/stretchr/testify".to_string(),
            alias: None,
        };
        assert!(resolver.is_external_dependency(&external_import));

        let other_external = crate::parser::Import {
            path: "github.com/other/library".to_string(),
            alias: None,
        };
        assert!(resolver.is_external_dependency(&other_external));

        // 测试标准库
        let std_import = crate::parser::Import {
            path: "fmt".to_string(),
            alias: None,
        };
        assert!(resolver.is_external_dependency(&std_import));
    }

    #[test]
    fn test_dependency_resolver_from_project_root_no_go_mod() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // 不创建 go.mod 文件
        let resolver = DependencyResolver::from_project_root(temp_dir.path())
            .expect("Failed to create resolver from project root");

        // 应该回退到默认行为
        let import = crate::parser::Import {
            path: "github.com/user/repo".to_string(),
            alias: None,
        };
        assert!(resolver.is_external_dependency(&import));

        let relative_import = crate::parser::Import {
            path: "./internal".to_string(),
            alias: None,
        };
        assert!(!resolver.is_external_dependency(&relative_import));
    }
}
