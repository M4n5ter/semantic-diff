//! 代码生成模块
//!
//! 提供代码切片生成和格式化功能

use crate::error::Result;
use crate::extractor::{ChangeTarget, SemanticContext};
use crate::git::{DiffHunk, DiffLineType};
use crate::parser::{GoFunctionInfo, GoTypeDefinition, Import};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// 代码切片生成器
pub struct CodeSliceGenerator {
    formatter: CodeFormatter,
    config: GeneratorConfig,
}

/// 代码格式化器
pub struct CodeFormatter {
    output_format: OutputFormat,
}

/// 生成器配置
#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    /// 是否包含注释
    pub include_comments: bool,
    /// 是否包含导入声明
    pub include_imports: bool,
    /// 是否包含类型定义
    pub include_types: bool,
    /// 是否包含依赖函数
    pub include_dependent_functions: bool,
    /// 是否包含依赖图
    pub include_dependency_graph: bool,
    /// 最大行数限制
    pub max_lines: Option<usize>,
    /// 输出格式
    pub output_format: OutputFormat,
    /// 高亮样式
    pub highlight_style: HighlightStyle,
}

/// 代码切片
#[derive(Debug, Clone)]
pub struct CodeSlice {
    /// 头部注释，描述代码切片的内容
    pub header_comment: String,
    /// 导入声明列表
    pub imports: Vec<String>,
    /// 类型定义列表
    pub type_definitions: Vec<String>,
    /// 函数定义列表
    pub function_definitions: Vec<String>,
    /// 常量定义列表
    pub constants: Vec<String>,
    /// 变量定义列表
    pub variables: Vec<String>,
    /// 高亮显示的行号（相对于生成的代码切片）
    pub highlighted_lines: Vec<u32>,
    /// 变更行的映射：原始行号 -> 切片中的行号
    pub line_mapping: HashMap<u32, u32>,
    /// 行号到变更类型的映射
    pub line_change_types: HashMap<u32, DiffLineType>,
    /// 涉及的文件路径
    pub involved_files: Vec<PathBuf>,
    /// 生成的完整代码内容
    pub content: String,
    /// 依赖图
    pub dependency_graph: Option<crate::extractor::DependencyGraph>,
}

/// 输出格式
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OutputFormat {
    PlainText,
    Markdown,
    Html,
}

/// 高亮样式
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HighlightStyle {
    None,
    Inline,
    Separate,
}

/// 代码行信息
#[derive(Debug, Clone)]
struct CodeLine {
    content: String,
    line_number: u32,
    is_highlighted: bool,
    change_type: Option<DiffLineType>,
}

/// 代码块信息
#[derive(Debug, Clone)]
struct CodeBlock {
    title: String,
    lines: Vec<CodeLine>,
    block_type: BlockType,
}

/// 代码块类型
#[derive(Debug, Clone, PartialEq, Eq)]
enum BlockType {
    Import,
    Type,
    Function,
    Constant,
    Variable,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            include_comments: true,
            include_imports: true,
            include_types: true,
            include_dependent_functions: true,
            include_dependency_graph: false,
            max_lines: None,
            output_format: OutputFormat::PlainText,
            highlight_style: HighlightStyle::Inline,
        }
    }
}

impl Default for CodeSliceGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeSliceGenerator {
    /// 创建新的代码切片生成器
    pub fn new() -> Self {
        Self::with_config(GeneratorConfig::default())
    }

    /// 使用指定配置创建代码切片生成器
    pub fn with_config(config: GeneratorConfig) -> Self {
        let formatter = CodeFormatter::new(config.output_format.clone());
        Self { formatter, config }
    }

    /// 生成代码切片
    pub fn generate_slice(
        &self,
        context: &SemanticContext,
        changes: &[DiffHunk],
    ) -> Result<CodeSlice> {
        let mut code_blocks = Vec::new();
        let mut involved_files = HashSet::new();

        // 1. 生成头部注释
        let header_comment = self.generate_header_comment(context);

        // 2. 生成导入声明块
        if self.config.include_imports && !context.imports.is_empty() {
            let import_block = self.generate_import_block(&context.imports);
            code_blocks.push(import_block);
        }

        // 3. 生成类型定义块
        if self.config.include_types {
            for type_def in &context.related_types {
                involved_files.insert(type_def.file_path.clone());
                let type_block = self.generate_type_block(type_def);
                code_blocks.push(type_block);
            }
        }

        // 4. 生成常量定义块
        for constant in &context.constants {
            involved_files.insert(constant.file_path.clone());
            let const_block = self.generate_constant_block(constant);
            code_blocks.push(const_block);
        }

        // 5. 生成变量定义块
        for variable in &context.variables {
            involved_files.insert(variable.file_path.clone());
            let var_block = self.generate_variable_block(variable);
            code_blocks.push(var_block);
        }

        // 6. 生成主要变更目标块
        let main_block = self.generate_change_target_block(&context.change_target);
        involved_files.insert(context.change_target.file_path().clone());
        code_blocks.push(main_block);

        // 7. 生成依赖函数块
        if self.config.include_dependent_functions {
            for function in &context.dependent_functions {
                involved_files.insert(function.file_path.clone());
                let func_block = self.generate_function_block(function);
                code_blocks.push(func_block);
            }
        }

        // 8. 应用变更高亮
        self.apply_change_highlighting(&mut code_blocks, changes)?;

        // 9. 生成依赖图
        let dependency_graph = if self.config.include_dependency_graph {
            Some(context.generate_dependency_graph())
        } else {
            None
        };

        // 10. 生成最终的代码切片（包含 diff 信息）
        let mut code_slice = self.build_code_slice_with_diff(
            header_comment,
            code_blocks,
            involved_files.into_iter().collect(),
            changes,
        )?;

        // 添加依赖图
        code_slice.dependency_graph = dependency_graph;

        Ok(code_slice)
    }

    /// 高亮变更
    pub fn highlight_changes(&self, slice: &mut CodeSlice, changes: &[DiffHunk]) -> Result<()> {
        // 清空现有的高亮信息
        slice.highlighted_lines.clear();
        slice.line_mapping.clear();

        // 重新计算高亮行
        let mut _current_line = 1u32;
        let lines: Vec<&str> = slice.content.lines().collect();

        for (line_index, line_content) in lines.iter().enumerate() {
            let line_num = (line_index + 1) as u32;

            // 检查这一行是否应该被高亮
            if self.should_highlight_line(line_content, changes) {
                slice.highlighted_lines.push(line_num);
            }

            _current_line += 1;
        }

        // 按行号排序
        slice.highlighted_lines.sort_unstable();

        Ok(())
    }

    /// 生成头部注释
    fn generate_header_comment(&self, context: &SemanticContext) -> String {
        let stats = context.get_stats();
        let change_type = match &context.change_target {
            ChangeTarget::Function(f) => format!("Function: {}", f.name),
            ChangeTarget::Type(t) => format!("Type: {}", t.name),
            ChangeTarget::Variable(v) => format!("Variable: {}", v.name),
            ChangeTarget::Constant(c) => format!("Constant: {}", c.name),
        };

        format!(
            "// Semantic Context for {}\n// Generated by semantic-diff\n// \n// Context includes:\n//   - {} types\n//   - {} functions\n//   - {} constants\n//   - {} variables\n//   - {} imports\n//   - {} files\n",
            change_type,
            stats.types_count,
            stats.functions_count,
            stats.constants_count,
            stats.variables_count,
            stats.imports_count,
            stats.files_count
        )
    }

    /// 生成导入声明块
    fn generate_import_block(&self, imports: &[Import]) -> CodeBlock {
        let mut lines = Vec::new();
        let mut line_number = 1u32;

        // 按路径排序导入
        let mut sorted_imports = imports.to_vec();
        sorted_imports.sort_by(|a, b| a.path.cmp(&b.path));

        for import in sorted_imports {
            let import_line = if let Some(alias) = &import.alias {
                format!("import {} \"{}\"", alias, import.path)
            } else {
                format!("import \"{}\"", import.path)
            };

            lines.push(CodeLine {
                content: import_line,
                line_number,
                is_highlighted: false,
                change_type: None,
            });
            line_number += 1;
        }

        CodeBlock {
            title: "Imports".to_string(),
            lines,
            block_type: BlockType::Import,
        }
    }

    /// 生成类型定义块
    fn generate_type_block(&self, type_def: &GoTypeDefinition) -> CodeBlock {
        let lines = self.split_into_lines(&type_def.definition, 1);

        CodeBlock {
            title: format!("Type: {}", type_def.name),
            lines,
            block_type: BlockType::Type,
        }
    }

    /// 生成常量定义块
    fn generate_constant_block(&self, constant: &crate::parser::GoConstantDefinition) -> CodeBlock {
        // 根据常量信息构建定义字符串
        let definition = if let Some(const_type) = &constant.const_type {
            format!(
                "const {} {} = {}",
                constant.name, const_type.name, constant.value
            )
        } else {
            format!("const {} = {}", constant.name, constant.value)
        };

        let lines = self.split_into_lines(&definition, constant.start_line);

        CodeBlock {
            title: format!("Constant: {}", constant.name),
            lines,
            block_type: BlockType::Constant,
        }
    }

    /// 生成变量定义块
    fn generate_variable_block(&self, variable: &crate::parser::GoVariableDefinition) -> CodeBlock {
        // 根据变量信息构建定义字符串
        let definition = match (&variable.var_type, &variable.initial_value) {
            (Some(var_type), Some(initial_value)) => {
                format!(
                    "var {} {} = {}",
                    variable.name, var_type.name, initial_value
                )
            }
            (Some(var_type), None) => {
                format!("var {} {}", variable.name, var_type.name)
            }
            (None, Some(initial_value)) => {
                format!("var {} = {}", variable.name, initial_value)
            }
            (None, None) => {
                format!("var {}", variable.name)
            }
        };

        let lines = self.split_into_lines(&definition, variable.start_line);

        CodeBlock {
            title: format!("Variable: {}", variable.name),
            lines,
            block_type: BlockType::Variable,
        }
    }

    /// 生成变更目标块
    fn generate_change_target_block(&self, change_target: &ChangeTarget) -> CodeBlock {
        match change_target {
            ChangeTarget::Function(func) => self.generate_function_block(func),
            ChangeTarget::Type(type_def) => self.generate_type_block(type_def),
            ChangeTarget::Variable(var) => self.generate_variable_block(var),
            ChangeTarget::Constant(const_def) => self.generate_constant_block(const_def),
        }
    }

    /// 生成函数定义块
    fn generate_function_block(&self, function: &GoFunctionInfo) -> CodeBlock {
        // 构建完整的函数定义，包括签名和函数体
        let full_function_definition = self.build_complete_function_definition(function);
        let lines = self.split_into_lines(&full_function_definition, function.start_line);

        CodeBlock {
            title: format!("Function: {}", function.name),
            lines,
            block_type: BlockType::Function,
        }
    }

    /// 构建完整的函数定义（包括签名和函数体）
    fn build_complete_function_definition(&self, function: &GoFunctionInfo) -> String {
        let mut definition = String::new();

        // 1. 构建函数签名
        let signature = self.build_function_signature(function);
        definition.push_str(&signature);

        // 2. 检查函数体是否已经包含完整的大括号结构
        let body = function.body.trim();
        if body.is_empty() {
            // 空函数体
            definition.push_str(" {\n}");
        } else if body.starts_with('{') && body.ends_with('}') {
            // 函数体已经包含完整的大括号结构
            definition.push(' ');
            definition.push_str(body);
        } else {
            // 函数体不包含大括号，需要添加
            definition.push_str(" {\n");

            // 添加函数体（缩进处理）
            for line in body.lines() {
                if !line.trim().is_empty() {
                    definition.push_str("    "); // 4个空格缩进
                    definition.push_str(line);
                }
                definition.push('\n');
            }

            // 添加结束大括号
            definition.push('}');
        }

        definition
    }

    /// 构建函数签名
    fn build_function_signature(&self, function: &GoFunctionInfo) -> String {
        let mut signature = String::new();

        // 1. 添加 func 关键字
        signature.push_str("func ");

        // 2. 添加接收者（如果是方法）
        if let Some(receiver) = &function.receiver {
            signature.push('(');
            signature.push_str(&receiver.name);
            signature.push(' ');
            if receiver.is_pointer {
                signature.push('*');
            }
            signature.push_str(&receiver.type_name);
            signature.push_str(") ");
        }

        // 3. 添加函数名
        signature.push_str(&function.name);

        // 4. 添加参数列表
        signature.push('(');
        for (i, param) in function.parameters.iter().enumerate() {
            if i > 0 {
                signature.push_str(", ");
            }
            signature.push_str(&param.name);
            signature.push(' ');
            if param.param_type.is_pointer {
                signature.push('*');
            }
            if param.param_type.is_slice {
                signature.push_str("[]");
            }
            signature.push_str(&param.param_type.name);
        }
        signature.push(')');

        // 5. 添加返回类型
        if !function.return_types.is_empty() {
            signature.push(' ');
            if function.return_types.len() == 1 {
                let return_type = &function.return_types[0];
                if return_type.is_pointer {
                    signature.push('*');
                }
                if return_type.is_slice {
                    signature.push_str("[]");
                }
                signature.push_str(&return_type.name);
            } else {
                signature.push('(');
                for (i, return_type) in function.return_types.iter().enumerate() {
                    if i > 0 {
                        signature.push_str(", ");
                    }
                    if return_type.is_pointer {
                        signature.push('*');
                    }
                    if return_type.is_slice {
                        signature.push_str("[]");
                    }
                    signature.push_str(&return_type.name);
                }
                signature.push(')');
            }
        }

        signature
    }

    /// 将文本分割为代码行
    fn split_into_lines(&self, content: &str, start_line: u32) -> Vec<CodeLine> {
        content
            .lines()
            .enumerate()
            .map(|(index, line)| CodeLine {
                content: line.to_string(),
                line_number: start_line + index as u32,
                is_highlighted: false,
                change_type: None,
            })
            .collect()
    }

    /// 应用变更高亮
    fn apply_change_highlighting(
        &self,
        code_blocks: &mut [CodeBlock],
        changes: &[DiffHunk],
    ) -> Result<()> {
        // 收集所有变更行，包括删除和添加的行
        let mut added_lines = Vec::new();
        let mut removed_lines = Vec::new();

        for change_hunk in changes {
            for diff_line in &change_hunk.lines {
                let clean_content = diff_line.content.trim_start_matches(['+', '-', ' ']).trim();

                // 跳过空行和过短的内容
                if clean_content.is_empty() || clean_content.len() <= 2 {
                    continue;
                }

                match diff_line.line_type {
                    DiffLineType::Added => {
                        added_lines.push(clean_content.to_string());
                    }
                    DiffLineType::Removed => {
                        removed_lines.push(clean_content.to_string());
                    }
                    DiffLineType::Context => {
                        // 上下文行不需要特殊处理
                    }
                }
            }
        }

        // 在代码块中查找匹配的行并标记变更类型
        for block in code_blocks.iter_mut() {
            for line in block.lines.iter_mut() {
                let line_content = line.content.trim();

                // 检查是否是添加的行
                if added_lines.contains(&line_content.to_string()) {
                    line.is_highlighted = true;
                    line.change_type = Some(DiffLineType::Added);
                }
                // 检查是否是删除的行（虽然删除的行通常不在新版本中，但可能在上下文中）
                else if removed_lines.contains(&line_content.to_string()) {
                    line.is_highlighted = true;
                    line.change_type = Some(DiffLineType::Removed);
                }
            }
        }

        Ok(())
    }

    /// 检查变更是否足够重要以进行高亮
    fn is_significant_change(&self, change_content: &str) -> bool {
        // 允许空行，因为它们在 diff 中可能很重要
        if change_content.is_empty() {
            return true;
        }

        // 跳过过短的内容，但允许一些重要的短内容
        if change_content.len() < 3 {
            return false;
        }

        // 跳过过于通用的内容
        let generic_patterns = [
            "return", "break", "continue", "}", "{", ")", "(", "nil", "null", "true", "false",
        ];

        if generic_patterns.contains(&change_content) {
            return false;
        }

        // 跳过只包含符号的行
        if change_content
            .chars()
            .all(|c| "{}()[];,".contains(c) || c.is_whitespace())
        {
            return false;
        }

        // 其他内容认为是重要的
        true
    }

    /// 检查行是否应该被高亮
    fn should_highlight_line(&self, line_content: &str, changes: &[DiffHunk]) -> bool {
        for change_hunk in changes {
            for diff_line in &change_hunk.lines {
                if matches!(
                    diff_line.line_type,
                    DiffLineType::Added | DiffLineType::Removed
                ) {
                    let change_content =
                        diff_line.content.trim_start_matches(['+', '-', ' ']).trim();
                    if line_content.trim() == change_content {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// 构建最终的代码切片
    #[allow(dead_code)]
    fn build_code_slice(
        &self,
        header_comment: String,
        code_blocks: Vec<CodeBlock>,
        involved_files: Vec<PathBuf>,
    ) -> Result<CodeSlice> {
        let mut content_parts = Vec::new();
        let mut imports = Vec::new();
        let mut type_definitions = Vec::new();
        let mut function_definitions = Vec::new();
        let mut constants = Vec::new();
        let mut variables = Vec::new();
        let mut highlighted_lines = Vec::new();
        let mut line_mapping = HashMap::new();
        let mut line_change_types = HashMap::new();
        let mut current_line = 1u32;

        // 添加头部注释
        content_parts.push(header_comment.clone());
        current_line += header_comment.lines().count() as u32;

        // 处理每个代码块
        for block in code_blocks {
            // 添加块标题注释
            let block_comment = format!("\n// {}\n", block.title);
            content_parts.push(block_comment.clone());
            current_line += block_comment.lines().count() as u32;

            // 处理块中的每一行
            let mut block_content = String::new();
            for line in &block.lines {
                block_content.push_str(&line.content);
                block_content.push('\n');

                // 记录行映射
                line_mapping.insert(line.line_number, current_line);

                // 记录高亮行和变更类型
                if line.is_highlighted {
                    highlighted_lines.push(current_line);
                    if let Some(change_type) = &line.change_type {
                        line_change_types.insert(current_line, change_type.clone());
                    }
                }

                current_line += 1;
            }

            content_parts.push(block_content.clone());

            // 按类型分类存储
            match block.block_type {
                BlockType::Import => imports.push(block_content),
                BlockType::Type => type_definitions.push(block_content),
                BlockType::Function => function_definitions.push(block_content),
                BlockType::Constant => constants.push(block_content),
                BlockType::Variable => variables.push(block_content),
            }
        }

        // 生成最终内容
        let raw_content = content_parts.join("");
        let content = self.formatter.format_content(&raw_content)?;

        let code_slice = CodeSlice {
            header_comment,
            imports,
            type_definitions,
            function_definitions,
            constants,
            variables,
            highlighted_lines,
            line_mapping,
            line_change_types,
            involved_files,
            content,
            dependency_graph: None, // 将在 generate_slice 中设置
        };

        Ok(code_slice)
    }

    /// 从 diff hunks 中构建包含删除和添加行的代码切片
    fn build_code_slice_with_diff(
        &self,
        header_comment: String,
        code_blocks: Vec<CodeBlock>,
        involved_files: Vec<PathBuf>,
        diff_hunks: &[DiffHunk],
    ) -> Result<CodeSlice> {
        let mut content_parts = Vec::new();
        let mut imports = Vec::new();
        let mut type_definitions = Vec::new();
        let mut function_definitions = Vec::new();
        let mut constants = Vec::new();
        let mut variables = Vec::new();
        let mut highlighted_lines = Vec::new();
        let mut line_mapping = HashMap::new();
        let mut line_change_types = HashMap::new();
        let mut current_line = 1u32;

        // 添加头部注释
        content_parts.push(header_comment.clone());
        current_line += header_comment.lines().count() as u32;

        // 处理每个代码块，直接从 diff 构建内容
        for block in code_blocks {
            // 添加块标题注释
            let block_comment = format!("\n// {}\n", block.title);
            content_parts.push(block_comment.clone());
            current_line += block_comment.lines().count() as u32;

            // 构建包含 diff 信息的块内容
            let mut block_content = String::new();
            let diff_aware_lines = self.build_diff_aware_lines(&block, diff_hunks)?;

            for line in &diff_aware_lines {
                block_content.push_str(&line.content);
                block_content.push('\n');

                // 记录行映射
                line_mapping.insert(line.line_number, current_line);

                // 记录高亮行和变更类型
                if line.is_highlighted {
                    highlighted_lines.push(current_line);
                    if let Some(change_type) = &line.change_type {
                        line_change_types.insert(current_line, change_type.clone());
                    }
                }

                current_line += 1;
            }

            content_parts.push(block_content.clone());

            // 按类型分类存储
            match block.block_type {
                BlockType::Import => imports.push(block_content),
                BlockType::Type => type_definitions.push(block_content),
                BlockType::Function => function_definitions.push(block_content),
                BlockType::Constant => constants.push(block_content),
                BlockType::Variable => variables.push(block_content),
            }
        }

        // 生成最终内容
        let raw_content = content_parts.join("");
        let content = self.formatter.format_content(&raw_content)?;

        let code_slice = CodeSlice {
            header_comment,
            imports,
            type_definitions,
            function_definitions,
            constants,
            variables,
            highlighted_lines,
            line_mapping,
            line_change_types,
            involved_files,
            content,
            dependency_graph: None, // 将在 generate_slice 中设置
        };

        Ok(code_slice)
    }

    /// 构建包含 diff 信息的行，避免重复
    fn build_diff_aware_lines(
        &self,
        block: &CodeBlock,
        diff_hunks: &[DiffHunk],
    ) -> Result<Vec<CodeLine>> {
        let mut result_lines = Vec::new();
        let mut line_counter = 1u32;

        // 收集所有 diff 行并按位置排序
        let mut all_diff_lines = Vec::new();
        for hunk in diff_hunks {
            for diff_line in &hunk.lines {
                all_diff_lines.push(diff_line);
            }
        }

        // 创建一个集合来跟踪已经处理的内容，避免重复
        let mut processed_content = HashSet::new();

        // 直接从 diff 中按顺序处理所有行
        for diff_line in &all_diff_lines {
            let clean_content = diff_line.content.trim_start_matches(['+', '-', ' ']).trim();

            // 避免重复添加相同内容
            if processed_content.contains(clean_content) {
                continue;
            }

            // 检查这个 diff 行是否与当前代码块相关
            let is_relevant = block.lines.iter().any(|block_line| {
                let block_content = block_line.content.trim();
                block_content == clean_content
                    || self.is_content_related(block_content, clean_content)
            });

            if !is_relevant {
                continue;
            }

            match diff_line.line_type {
                DiffLineType::Removed => {
                    result_lines.push(CodeLine {
                        content: clean_content.to_string(),
                        line_number: line_counter,
                        is_highlighted: true,
                        change_type: Some(DiffLineType::Removed),
                    });
                    processed_content.insert(clean_content.to_string());
                    line_counter += 1;
                }
                DiffLineType::Added => {
                    result_lines.push(CodeLine {
                        content: clean_content.to_string(),
                        line_number: line_counter,
                        is_highlighted: true,
                        change_type: Some(DiffLineType::Added),
                    });
                    processed_content.insert(clean_content.to_string());
                    line_counter += 1;
                }
                DiffLineType::Context => {
                    // 上下文行不需要特殊标记，但仍然添加
                    result_lines.push(CodeLine {
                        content: clean_content.to_string(),
                        line_number: line_counter,
                        is_highlighted: false,
                        change_type: None,
                    });
                    processed_content.insert(clean_content.to_string());
                    line_counter += 1;
                }
            }
        }

        // 如果没有找到相关的 diff 行，回退到原始代码块
        if result_lines.is_empty() {
            for original_line in &block.lines {
                let mut updated_line = original_line.clone();
                updated_line.line_number = line_counter;
                result_lines.push(updated_line);
                line_counter += 1;
            }
        }

        Ok(result_lines)
    }

    /// 增强代码块，包含 diff 中的删除和添加行
    #[allow(dead_code)]
    fn enhance_block_with_diff(
        &self,
        block: &CodeBlock,
        diff_hunks: &[DiffHunk],
    ) -> Result<Vec<CodeLine>> {
        let mut enhanced_lines = Vec::new();
        let mut line_counter = 1u32;

        // 收集所有相关的 diff 行，按行号排序
        let mut diff_lines = Vec::new();
        for hunk in diff_hunks {
            for diff_line in &hunk.lines {
                diff_lines.push(diff_line);
            }
        }

        // 按新行号排序（如果有的话）
        diff_lines.sort_by_key(|line| line.new_line_number.unwrap_or(u32::MAX));

        // 创建一个映射，将原始行内容映射到 diff 行
        let mut content_to_diff = std::collections::HashMap::new();
        for diff_line in &diff_lines {
            let clean_content = diff_line.content.trim_start_matches(['+', '-', ' ']).trim();
            if !clean_content.is_empty() {
                content_to_diff.insert(clean_content.to_string(), diff_line);
            }
        }

        // 处理每个原始行，同时插入相关的删除行
        for original_line in &block.lines {
            let original_content = original_line.content.trim();

            // 首先查找是否有相关的删除行需要在此行之前显示
            for diff_line in &diff_lines {
                if diff_line.line_type == DiffLineType::Removed {
                    let diff_content = diff_line.content.trim_start_matches(['+', '-', ' ']).trim();

                    // 如果删除的行与当前行相关（例如，被当前行替换）
                    if self.is_content_related(original_content, diff_content) {
                        enhanced_lines.push(CodeLine {
                            content: diff_content.to_string(),
                            line_number: line_counter,
                            is_highlighted: true,
                            change_type: Some(DiffLineType::Removed),
                        });
                        line_counter += 1;
                    }
                }
            }

            // 添加原始行，检查它是否是添加的行
            let mut change_type = original_line.change_type.clone();
            let mut is_highlighted = original_line.is_highlighted;

            if let Some(diff_line) = content_to_diff.get(original_content) {
                match diff_line.line_type {
                    DiffLineType::Added => {
                        change_type = Some(DiffLineType::Added);
                        is_highlighted = true;
                    }
                    DiffLineType::Removed => {
                        change_type = Some(DiffLineType::Removed);
                        is_highlighted = true;
                    }
                    DiffLineType::Context => {
                        // 上下文行不需要特殊处理
                    }
                }
            }

            enhanced_lines.push(CodeLine {
                content: original_line.content.clone(),
                line_number: line_counter,
                is_highlighted,
                change_type,
            });
            line_counter += 1;
        }

        // 最后，添加任何没有匹配到原始行的删除行
        for diff_line in &diff_lines {
            if diff_line.line_type == DiffLineType::Removed {
                let diff_content = diff_line.content.trim_start_matches(['+', '-', ' ']).trim();

                // 检查这个删除行是否已经被添加
                let already_added = enhanced_lines.iter().any(|line| {
                    line.content.trim() == diff_content
                        && matches!(line.change_type, Some(DiffLineType::Removed))
                });

                if !already_added && self.is_significant_change(diff_content) {
                    enhanced_lines.push(CodeLine {
                        content: diff_content.to_string(),
                        line_number: line_counter,
                        is_highlighted: true,
                        change_type: Some(DiffLineType::Removed),
                    });
                    line_counter += 1;
                }
            }
        }

        Ok(enhanced_lines)
    }

    /// 检查两个内容是否相关（用于匹配删除和添加的行）
    fn is_content_related(&self, content1: &str, content2: &str) -> bool {
        // 直接相等
        if content1 == content2 {
            return true;
        }

        // 如果其中一个是空行，检查另一个是否也是空行或接近空行
        if content1.trim().is_empty() || content2.trim().is_empty() {
            return content1.trim().is_empty() && content2.trim().is_empty();
        }

        // 检查是否包含相同的关键词
        let words1: HashSet<&str> = content1.split_whitespace().collect();
        let words2: HashSet<&str> = content2.split_whitespace().collect();

        if words1.is_empty() || words2.is_empty() {
            return false;
        }

        let intersection_count = words1.intersection(&words2).count();
        let union_count = words1.union(&words2).count();

        // 如果有超过30%的词汇重叠，认为是相关的（降低阈值）
        union_count > 0 && (intersection_count as f64 / union_count as f64) > 0.3
    }
}

impl Default for CodeFormatter {
    fn default() -> Self {
        Self::new(OutputFormat::PlainText)
    }
}

impl CodeFormatter {
    /// 创建新的代码格式化器
    pub fn new(output_format: OutputFormat) -> Self {
        Self { output_format }
    }

    /// 格式化内容
    pub fn format_content(&self, content: &str) -> Result<String> {
        match self.output_format {
            OutputFormat::PlainText => Ok(self.format_plain_text(content)),
            OutputFormat::Markdown => Ok(self.format_markdown(content)),
            OutputFormat::Html => Ok(self.format_html(content)),
        }
    }

    /// 格式化为纯文本
    fn format_plain_text(&self, content: &str) -> String {
        // 对于纯文本，直接返回内容
        content.to_string()
    }

    /// 格式化为 Markdown
    fn format_markdown(&self, content: &str) -> String {
        // 在 Markdown 渲染中，代码块标记由 formatter.rs 处理
        // 这里只返回内容本身
        content.to_string()
    }

    /// 格式化为 HTML
    fn format_html(&self, content: &str) -> String {
        let mut result = String::new();
        result.push_str("<pre><code class=\"language-go\">\n");

        // HTML 转义
        let escaped_content = content
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;");

        result.push_str(&escaped_content);
        result.push_str("</code></pre>\n");
        result
    }
}

impl CodeSlice {
    /// 获取格式化后的内容（不包含高亮，高亮由 formatter 处理）
    pub fn get_formatted_content(&self, formatter: &CodeFormatter) -> Result<String> {
        formatter.format_content(&self.content)
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> CodeSliceStats {
        CodeSliceStats {
            total_lines: self.content.lines().count(),
            highlighted_lines: self.highlighted_lines.len(),
            imports_count: self.imports.len(),
            types_count: self.type_definitions.len(),
            functions_count: self.function_definitions.len(),
            constants_count: self.constants.len(),
            variables_count: self.variables.len(),
            files_count: self.involved_files.len(),
        }
    }

    /// 检查是否包含高亮内容
    pub fn has_highlights(&self) -> bool {
        !self.highlighted_lines.is_empty()
    }

    /// 获取高亮行的内容
    pub fn get_highlighted_content(&self) -> Vec<String> {
        let lines: Vec<&str> = self.content.lines().collect();
        self.highlighted_lines
            .iter()
            .filter_map(|&line_num| {
                lines
                    .get((line_num - 1) as usize)
                    .map(|line| line.to_string())
            })
            .collect()
    }
}

/// 代码切片统计信息
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeSliceStats {
    pub total_lines: usize,
    pub highlighted_lines: usize,
    pub imports_count: usize,
    pub types_count: usize,
    pub functions_count: usize,
    pub constants_count: usize,
    pub variables_count: usize,
    pub files_count: usize,
}

#[cfg(test)]
mod tests;
