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
    highlight_style: HighlightStyle,
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
        let formatter =
            CodeFormatter::new(config.output_format.clone(), config.highlight_style.clone());
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

        // 10. 生成最终的代码切片
        let mut code_slice = self.build_code_slice(
            header_comment,
            code_blocks,
            involved_files.into_iter().collect(),
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
        for change_hunk in changes {
            for diff_line in &change_hunk.lines {
                let target_line = match diff_line.line_type {
                    DiffLineType::Added => {
                        change_hunk.new_start + diff_line.content.lines().count() as u32 - 1
                    }
                    DiffLineType::Removed => {
                        change_hunk.old_start + diff_line.content.lines().count() as u32 - 1
                    }
                    DiffLineType::Context => continue,
                };

                // 在代码块中查找对应的行并标记高亮
                for block in code_blocks.iter_mut() {
                    for line in &mut block.lines {
                        if self.line_matches_change(line, &diff_line.content, target_line) {
                            line.is_highlighted = true;
                            line.change_type = Some(diff_line.line_type.clone());
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 检查行是否匹配变更
    fn line_matches_change(
        &self,
        line: &CodeLine,
        change_content: &str,
        _target_line: u32,
    ) -> bool {
        // 简单的内容匹配，去除前后空白
        let line_trimmed = line.content.trim();
        let change_trimmed = change_content.trim_start_matches(['+', '-', ' ']);

        line_trimmed == change_trimmed.trim()
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

                // 记录高亮行
                if line.is_highlighted {
                    highlighted_lines.push(current_line);
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
        let content = self.formatter.format_content(&content_parts.join(""))?;

        Ok(CodeSlice {
            header_comment,
            imports,
            type_definitions,
            function_definitions,
            constants,
            variables,
            highlighted_lines,
            line_mapping,
            involved_files,
            content,
            dependency_graph: None, // 将在 generate_slice 中设置
        })
    }
}

impl Default for CodeFormatter {
    fn default() -> Self {
        Self::new(OutputFormat::PlainText, HighlightStyle::Inline)
    }
}

impl CodeFormatter {
    /// 创建新的代码格式化器
    pub fn new(output_format: OutputFormat, highlight_style: HighlightStyle) -> Self {
        Self {
            output_format,
            highlight_style,
        }
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
        let mut result = String::new();
        result.push_str("```go\n");
        result.push_str(content);
        if !content.ends_with('\n') {
            result.push('\n');
        }
        result.push_str("```\n");
        result
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

    /// 应用语法高亮
    pub fn apply_syntax_highlighting(
        &self,
        content: &str,
        highlighted_lines: &[u32],
    ) -> Result<String> {
        match self.highlight_style {
            HighlightStyle::None => Ok(content.to_string()),
            HighlightStyle::Inline => self.apply_inline_highlighting(content, highlighted_lines),
            HighlightStyle::Separate => {
                self.apply_separate_highlighting(content, highlighted_lines)
            }
        }
    }

    /// 应用内联高亮
    fn apply_inline_highlighting(
        &self,
        content: &str,
        highlighted_lines: &[u32],
    ) -> Result<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = String::new();

        for (index, line) in lines.iter().enumerate() {
            let line_number = (index + 1) as u32;

            if highlighted_lines.contains(&line_number) {
                match self.output_format {
                    OutputFormat::PlainText => {
                        result.push_str(&format!("> {line}\n"));
                    }
                    OutputFormat::Markdown => {
                        result.push_str(&format!("**{line}**\n"));
                    }
                    OutputFormat::Html => {
                        result.push_str(&format!("<mark>{line}</mark>\n"));
                    }
                }
            } else {
                result.push_str(line);
                result.push('\n');
            }
        }

        Ok(result)
    }

    /// 应用分离式高亮
    fn apply_separate_highlighting(
        &self,
        content: &str,
        highlighted_lines: &[u32],
    ) -> Result<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = String::new();
        let mut highlighted_content = String::new();

        // 先添加完整内容
        result.push_str(content);
        result.push_str("\n\n");

        // 然后添加高亮部分
        result.push_str("// Highlighted changes:\n");
        for &line_number in highlighted_lines {
            if let Some(line) = lines.get((line_number - 1) as usize) {
                highlighted_content.push_str(&format!("// Line {line_number}: {line}\n"));
            }
        }

        result.push_str(&highlighted_content);
        Ok(result)
    }
}

impl CodeSlice {
    /// 获取格式化后的内容
    pub fn get_formatted_content(&self, formatter: &CodeFormatter) -> Result<String> {
        formatter.apply_syntax_highlighting(&self.content, &self.highlighted_lines)
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
