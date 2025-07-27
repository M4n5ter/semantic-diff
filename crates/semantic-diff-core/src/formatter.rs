//! 输出格式化模块
//!
//! 提供多种输出格式支持和语法高亮功能

use crate::error::{Result, SemanticDiffError};
use crate::generator::{CodeSlice, HighlightStyle, OutputFormat};
use serde::{Deserialize, Serialize};

/// 输出格式化器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatterConfig {
    /// 输出格式
    pub output_format: OutputFormat,
    /// 高亮样式
    pub highlight_style: HighlightStyle,
    /// 是否显示行号
    pub show_line_numbers: bool,
    /// 是否显示文件路径
    pub show_file_paths: bool,
    /// 是否显示统计信息
    pub show_statistics: bool,
    /// 是否启用颜色输出（仅对终端输出有效）
    pub enable_colors: bool,
    /// 代码块标题样式
    pub block_title_style: BlockTitleStyle,
    /// 自定义CSS样式（仅对HTML输出有效）
    pub custom_css: Option<String>,
    /// 最大行宽
    pub max_line_width: Option<usize>,
    /// 缩进大小
    pub indent_size: usize,
}

/// 代码块标题样式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BlockTitleStyle {
    /// 简洁样式
    Minimal,
    /// 详细样式
    Detailed,
    /// 无标题
    None,
}

/// 输出渲染器
pub struct OutputRenderer {
    config: FormatterConfig,
}

/// 格式化结果
#[derive(Debug, Clone)]
pub struct FormattedOutput {
    /// 格式化后的内容
    pub content: String,
    /// 输出格式
    pub format: OutputFormat,
    /// 元数据
    pub metadata: OutputMetadata,
}

/// 输出元数据
#[derive(Debug, Clone)]
pub struct OutputMetadata {
    /// 总行数
    pub total_lines: usize,
    /// 高亮行数
    pub highlighted_lines: usize,
    /// 涉及的文件数
    pub files_count: usize,
    /// 生成时间戳
    pub generated_at: String,
    /// 内容大小（字节）
    pub content_size: usize,
}

/// 语法高亮器
pub struct SyntaxHighlighter {
    _language: String,
    _style: HighlightStyle,
}

/// 颜色主题
#[derive(Debug, Clone)]
pub struct ColorTheme {
    /// 添加行颜色
    pub added_line: String,
    /// 删除行颜色
    pub removed_line: String,
    /// 上下文行颜色
    pub context_line: String,
    /// 行号颜色
    pub line_number: String,
    /// 注释颜色
    pub comment: String,
    /// 关键字颜色
    pub keyword: String,
    /// 字符串颜色
    pub string: String,
    /// 数字颜色
    pub number: String,
}

impl Default for FormatterConfig {
    fn default() -> Self {
        Self {
            output_format: OutputFormat::PlainText,
            highlight_style: HighlightStyle::Inline,
            show_line_numbers: true,
            show_file_paths: true,
            show_statistics: true,
            enable_colors: true,
            block_title_style: BlockTitleStyle::Detailed,
            custom_css: None,
            max_line_width: Some(120),
            indent_size: 4,
        }
    }
}

impl Default for ColorTheme {
    fn default() -> Self {
        Self {
            added_line: "\x1b[32m".to_string(),   // 绿色
            removed_line: "\x1b[31m".to_string(), // 红色
            context_line: "\x1b[37m".to_string(), // 白色
            line_number: "\x1b[36m".to_string(),  // 青色
            comment: "\x1b[90m".to_string(),      // 灰色
            keyword: "\x1b[34m".to_string(),      // 蓝色
            string: "\x1b[33m".to_string(),       // 黄色
            number: "\x1b[35m".to_string(),       // 紫色
        }
    }
}

impl OutputRenderer {
    /// 创建新的输出渲染器
    pub fn new(config: FormatterConfig) -> Self {
        Self { config }
    }

    /// 使用默认配置创建渲染器
    pub fn with_default_config() -> Self {
        Self::new(FormatterConfig::default())
    }

    /// 渲染代码切片
    pub fn render(&self, code_slice: &CodeSlice) -> Result<FormattedOutput> {
        let content = match self.config.output_format {
            OutputFormat::PlainText => self.render_plain_text(code_slice)?,
            OutputFormat::Markdown => self.render_markdown(code_slice)?,
            OutputFormat::Html => self.render_html(code_slice)?,
        };

        let metadata = self.generate_metadata(code_slice, &content);

        Ok(FormattedOutput {
            content,
            format: self.config.output_format.clone(),
            metadata,
        })
    }

    /// 渲染为纯文本格式
    fn render_plain_text(&self, code_slice: &CodeSlice) -> Result<String> {
        let mut output = String::new();

        // 添加统计信息
        if self.config.show_statistics {
            output.push_str(&self.format_statistics(code_slice));
            output.push('\n');
            output.push('\n');
        }

        // 添加文件路径信息
        if self.config.show_file_paths && !code_slice.involved_files.is_empty() {
            output.push_str("Files involved:\n");
            for file_path in &code_slice.involved_files {
                output.push_str(&format!("  - {}\n", file_path.display()));
            }
            output.push('\n');
        }

        // 添加头部注释
        output.push_str(&code_slice.header_comment);
        output.push('\n');

        // 使用内置的高亮方法处理内容
        let highlighted_content = self.apply_highlighting_plain_text(code_slice)?;
        output.push_str(&highlighted_content);

        Ok(output)
    }

    /// 渲染为Markdown格式
    fn render_markdown(&self, code_slice: &CodeSlice) -> Result<String> {
        let mut output = String::new();

        // 添加标题
        output.push_str("# Semantic Diff Analysis\n\n");

        // 添加统计信息
        if self.config.show_statistics {
            output.push_str("## Statistics\n\n");
            output.push_str(&self.format_statistics_markdown(code_slice));
            output.push_str("\n\n");
        }

        // 添加文件路径信息
        if self.config.show_file_paths && !code_slice.involved_files.is_empty() {
            output.push_str("## Files Involved\n\n");
            for file_path in &code_slice.involved_files {
                output.push_str(&format!("- `{}`\n", file_path.display()));
            }
            output.push('\n');
        }

        // 添加代码块
        output.push_str("## Code Analysis\n\n");
        output.push_str(
            &code_slice
                .header_comment
                .replace("//", "<!--")
                .replace('\n', "-->\n"),
        );
        output.push_str("\n\n");

        // 使用内置的高亮方法处理内容
        let highlighted_content = self.apply_highlighting_markdown(code_slice)?;

        // 将格式化内容包装在代码块中
        output.push_str("```go\n");
        output.push_str(&highlighted_content);
        output.push_str("\n```\n");

        Ok(output)
    }

    /// 渲染为HTML格式
    fn render_html(&self, code_slice: &CodeSlice) -> Result<String> {
        let mut output = String::new();

        // HTML文档头部
        output.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
        output.push_str("    <meta charset=\"UTF-8\">\n");
        output.push_str(
            "    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n",
        );
        output.push_str("    <title>Semantic Diff Analysis</title>\n");

        // 添加默认CSS样式
        output.push_str("    <style>\n");
        output.push_str(&self.get_default_css());

        // 添加自定义CSS
        if let Some(custom_css) = &self.config.custom_css {
            output.push_str(custom_css);
        }

        output.push_str("    </style>\n");
        output.push_str("</head>\n<body>\n");

        // 页面内容
        output.push_str("    <div class=\"container\">\n");
        output.push_str("        <h1>Semantic Diff Analysis</h1>\n");

        // 统计信息
        if self.config.show_statistics {
            output.push_str("        <div class=\"statistics\">\n");
            output.push_str("            <h2>Statistics</h2>\n");
            output.push_str(&self.format_statistics_html(code_slice));
            output.push_str("        </div>\n");
        }

        // 文件路径信息
        if self.config.show_file_paths && !code_slice.involved_files.is_empty() {
            output.push_str("        <div class=\"files\">\n");
            output.push_str("            <h2>Files Involved</h2>\n");
            output.push_str("            <ul>\n");
            for file_path in &code_slice.involved_files {
                output.push_str(&format!(
                    "                <li><code>{}</code></li>\n",
                    html_escape(&file_path.display().to_string())
                ));
            }
            output.push_str("            </ul>\n");
            output.push_str("        </div>\n");
        }

        // 代码分析
        output.push_str("        <div class=\"code-analysis\">\n");
        output.push_str("            <h2>Code Analysis</h2>\n");

        // 头部注释
        let escaped_comment = html_escape(&code_slice.header_comment);
        output.push_str(&format!("            <div class=\"header-comment\">\n                <pre>{escaped_comment}</pre>\n            </div>\n"));

        // 使用内置的高亮方法处理内容
        let highlighted_content = self.apply_highlighting_html(code_slice)?;

        output.push_str("            <div class=\"code-block\">\n");
        output.push_str(&highlighted_content);
        output.push_str("            </div>\n");
        output.push_str("        </div>\n");

        // HTML文档尾部
        output.push_str("    </div>\n");
        output.push_str("</body>\n</html>\n");

        Ok(output)
    }

    /// 应用纯文本高亮
    fn apply_highlighting_plain_text(&self, code_slice: &CodeSlice) -> Result<String> {
        match self.config.highlight_style {
            HighlightStyle::None => Ok(code_slice.content.clone()),
            HighlightStyle::Inline => self.apply_inline_highlighting_plain_text(code_slice),
            HighlightStyle::Separate => self.apply_separate_highlighting_plain_text(code_slice),
        }
    }

    /// 应用内联高亮（纯文本）
    fn apply_inline_highlighting_plain_text(&self, code_slice: &CodeSlice) -> Result<String> {
        let lines: Vec<&str> = code_slice.content.lines().collect();
        let mut result = String::new();

        for (index, line) in lines.iter().enumerate() {
            let line_number = (index + 1) as u32;
            let is_highlighted = code_slice.highlighted_lines.contains(&line_number);

            if is_highlighted {
                // 获取变更类型并显示相应的前缀
                let change_prefix =
                    if let Some(change_type) = code_slice.line_change_types.get(&line_number) {
                        match change_type {
                            crate::git::DiffLineType::Added => "+",
                            crate::git::DiffLineType::Removed => "-",
                            crate::git::DiffLineType::Context => ">",
                        }
                    } else {
                        ">"
                    };

                if self.config.enable_colors {
                    let color = match change_prefix {
                        "+" => "\x1b[32m", // 绿色用于添加
                        "-" => "\x1b[31m", // 红色用于删除
                        _ => "\x1b[33m",   // 黄色用于其他变更
                    };
                    result.push_str(&format!("{color}{change_prefix} {line}\x1b[0m\n"));
                } else {
                    result.push_str(&format!("{change_prefix} {line}\n"));
                }
            } else if self.config.show_line_numbers {
                if self.config.enable_colors {
                    result.push_str(&format!("\x1b[36m{line_number:4}|\x1b[0m {line}\n"));
                } else {
                    result.push_str(&format!("{line_number:4}| {line}\n"));
                }
            } else {
                result.push_str(&format!("{line}\n"));
            }
        }

        Ok(result)
    }

    /// 应用分离式高亮（纯文本）
    fn apply_separate_highlighting_plain_text(&self, code_slice: &CodeSlice) -> Result<String> {
        let lines: Vec<&str> = code_slice.content.lines().collect();
        let mut result = String::new();

        // 添加完整内容
        result.push_str("=== Full Content ===\n");
        result.push_str(&code_slice.content);
        result.push_str("\n\n");

        // 添加高亮部分
        result.push_str("=== Highlighted Changes ===\n");
        for &line_number in &code_slice.highlighted_lines {
            if let Some(line) = lines.get((line_number - 1) as usize) {
                let change_prefix =
                    if let Some(change_type) = code_slice.line_change_types.get(&line_number) {
                        match change_type {
                            crate::git::DiffLineType::Added => "+ ",
                            crate::git::DiffLineType::Removed => "- ",
                            crate::git::DiffLineType::Context => "> ",
                        }
                    } else {
                        "> "
                    };

                if self.config.enable_colors {
                    let color = match change_prefix.trim() {
                        "+" => "\x1b[32m", // 绿色用于添加
                        "-" => "\x1b[31m", // 红色用于删除
                        _ => "\x1b[33m",   // 黄色用于其他变更
                    };
                    result.push_str(&format!(
                        "{color}Line {line_number}: {change_prefix}{line}\x1b[0m\n"
                    ));
                } else {
                    result.push_str(&format!("Line {line_number}: {change_prefix}{line}\n"));
                }
            }
        }

        Ok(result)
    }

    /// 应用Markdown高亮
    fn apply_highlighting_markdown(&self, code_slice: &CodeSlice) -> Result<String> {
        match self.config.highlight_style {
            HighlightStyle::None => Ok(format!("```go\n{}\n```\n", code_slice.content)),
            HighlightStyle::Inline => self.apply_inline_highlighting_markdown(code_slice),
            HighlightStyle::Separate => self.apply_separate_highlighting_markdown(code_slice),
        }
    }

    /// 应用内联高亮（Markdown）
    fn apply_inline_highlighting_markdown(&self, code_slice: &CodeSlice) -> Result<String> {
        let lines: Vec<&str> = code_slice.content.lines().collect();
        let mut result = String::new();

        result.push_str("```go\n");

        for (index, line) in lines.iter().enumerate() {
            let line_number = (index + 1) as u32;
            let is_highlighted = code_slice.highlighted_lines.contains(&line_number);

            if is_highlighted {
                // 在Markdown中，我们使用注释来标记高亮行，显示变更类型
                let change_prefix =
                    if let Some(change_type) = code_slice.line_change_types.get(&line_number) {
                        match change_type {
                            crate::git::DiffLineType::Added => "// +++ ",
                            crate::git::DiffLineType::Removed => "// --- ",
                            crate::git::DiffLineType::Context => "// >>> ",
                        }
                    } else {
                        "// >>> "
                    };
                result.push_str(&format!("{change_prefix}{line}\n"));
            } else {
                result.push_str(&format!("{line}\n"));
            }
        }

        result.push_str("```\n");

        Ok(result)
    }

    /// 应用分离式高亮（Markdown）
    fn apply_separate_highlighting_markdown(&self, code_slice: &CodeSlice) -> Result<String> {
        let lines: Vec<&str> = code_slice.content.lines().collect();
        let mut result = String::new();

        // 添加完整代码部分
        result.push_str("### Full Code\n\n");
        result.push_str("```go\n");
        result.push_str(&code_slice.content);
        result.push_str("\n```\n\n");

        // 添加高亮部分
        result.push_str("### Highlighted Changes\n\n");
        result.push_str("```diff\n");
        for &line_number in &code_slice.highlighted_lines {
            if let Some(line) = lines.get((line_number - 1) as usize) {
                let change_prefix =
                    if let Some(change_type) = code_slice.line_change_types.get(&line_number) {
                        match change_type {
                            crate::git::DiffLineType::Added => "+",
                            crate::git::DiffLineType::Removed => "-",
                            crate::git::DiffLineType::Context => " ",
                        }
                    } else {
                        "+"
                    };
                result.push_str(&format!("{change_prefix}{line}\n"));
            }
        }
        result.push_str("```\n");

        Ok(result)
    }

    /// 应用HTML高亮
    fn apply_highlighting_html(&self, code_slice: &CodeSlice) -> Result<String> {
        let lines: Vec<&str> = code_slice.content.lines().collect();
        let mut result = String::new();

        result.push_str("<pre><code class=\"language-go\">\n");

        for (index, line) in lines.iter().enumerate() {
            let line_number = (index + 1) as u32;
            let is_highlighted = code_slice.highlighted_lines.contains(&line_number);

            if self.config.show_line_numbers {
                result.push_str(&format!(
                    "<span class=\"line-number\">{line_number:4}</span>"
                ));
            }

            if is_highlighted {
                let (css_class, change_prefix) =
                    if let Some(change_type) = code_slice.line_change_types.get(&line_number) {
                        match change_type {
                            crate::git::DiffLineType::Added => ("added-line", "+"),
                            crate::git::DiffLineType::Removed => ("removed-line", "-"),
                            crate::git::DiffLineType::Context => ("highlighted-line", " "),
                        }
                    } else {
                        ("highlighted-line", ">")
                    };

                result.push_str(&format!(
                    "<span class=\"{}\"><span class=\"change-prefix\">{}</span>{}</span>\n",
                    css_class,
                    change_prefix,
                    html_escape(line)
                ));
            } else {
                result.push_str(&format!("{}\n", html_escape(line)));
            }
        }

        result.push_str("</code></pre>\n");

        Ok(result)
    }

    /// 格式化统计信息
    fn format_statistics(&self, code_slice: &CodeSlice) -> String {
        let stats = code_slice.get_stats();
        format!(
            "Statistics:\n  Total lines: {}\n  Highlighted lines: {}\n  Files: {}\n  Imports: {}\n  Types: {}\n  Functions: {}\n  Constants: {}\n  Variables: {}",
            stats.total_lines,
            stats.highlighted_lines,
            stats.files_count,
            stats.imports_count,
            stats.types_count,
            stats.functions_count,
            stats.constants_count,
            stats.variables_count
        )
    }

    /// 格式化统计信息（Markdown）
    fn format_statistics_markdown(&self, code_slice: &CodeSlice) -> String {
        let stats = code_slice.get_stats();
        format!(
            "| Metric | Count |\n|--------|-------|\n| Total lines | {} |\n| Highlighted lines | {} |\n| Files | {} |\n| Imports | {} |\n| Types | {} |\n| Functions | {} |\n| Constants | {} |\n| Variables | {} |",
            stats.total_lines,
            stats.highlighted_lines,
            stats.files_count,
            stats.imports_count,
            stats.types_count,
            stats.functions_count,
            stats.constants_count,
            stats.variables_count
        )
    }

    /// 格式化统计信息（HTML）
    fn format_statistics_html(&self, code_slice: &CodeSlice) -> String {
        let stats = code_slice.get_stats();
        format!(
            "            <table class=\"stats-table\">\n\
             <tr><td>Total lines</td><td>{}</td></tr>\n\
             <tr><td>Highlighted lines</td><td>{}</td></tr>\n\
             <tr><td>Files</td><td>{}</td></tr>\n\
             <tr><td>Imports</td><td>{}</td></tr>\n\
             <tr><td>Types</td><td>{}</td></tr>\n\
             <tr><td>Functions</td><td>{}</td></tr>\n\
             <tr><td>Constants</td><td>{}</td></tr>\n\
             <tr><td>Variables</td><td>{}</td></tr>\n\
             </table>",
            stats.total_lines,
            stats.highlighted_lines,
            stats.files_count,
            stats.imports_count,
            stats.types_count,
            stats.functions_count,
            stats.constants_count,
            stats.variables_count
        )
    }

    /// 生成输出元数据
    fn generate_metadata(&self, code_slice: &CodeSlice, content: &str) -> OutputMetadata {
        OutputMetadata {
            total_lines: content.lines().count(),
            highlighted_lines: code_slice.highlighted_lines.len(),
            files_count: code_slice.involved_files.len(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            content_size: content.len(),
        }
    }

    /// 获取默认CSS样式
    fn get_default_css(&self) -> String {
        r#"
        body {
            font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
            line-height: 1.6;
            margin: 0;
            padding: 20px;
            background-color: #f8f9fa;
        }
        .container {
            max-width: 1200px;
            margin: 0 auto;
            background-color: white;
            padding: 30px;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        h1, h2 {
            color: #333;
            border-bottom: 2px solid #e9ecef;
            padding-bottom: 10px;
        }
        .statistics, .files, .code-analysis {
            margin: 20px 0;
        }
        .stats-table {
            border-collapse: collapse;
            width: 100%;
            margin: 10px 0;
        }
        .stats-table td {
            border: 1px solid #dee2e6;
            padding: 8px 12px;
        }
        .stats-table td:first-child {
            font-weight: bold;
            background-color: #f8f9fa;
        }
        .header-comment {
            background-color: #f8f9fa;
            border-left: 4px solid #007bff;
            padding: 15px;
            margin: 15px 0;
        }
        .code-block {
            background-color: #f8f9fa;
            border: 1px solid #e9ecef;
            border-radius: 4px;
            overflow-x: auto;
        }
        .code-block pre {
            margin: 0;
            padding: 15px;
        }
        .line-wrapper {
            display: block;
        }
        .line-number {
            color: #6c757d;
            margin-right: 15px;
            user-select: none;
        }
        .highlighted-line {
            background-color: #fff3cd;
            border-left: 3px solid #ffc107;
            padding-left: 5px;
        }
        .normal-line {
            padding-left: 8px;
        }
        ul {
            list-style-type: none;
            padding-left: 0;
        }
        ul li {
            padding: 5px 0;
            border-bottom: 1px solid #e9ecef;
        }
        ul li:last-child {
            border-bottom: none;
        }
        code {
            background-color: #f8f9fa;
            padding: 2px 4px;
            border-radius: 3px;
            font-family: inherit;
        }
        "#
        .to_string()
    }
}

impl SyntaxHighlighter {
    /// 创建新的语法高亮器
    pub fn new(language: String, style: HighlightStyle) -> Self {
        Self {
            _language: language,
            _style: style,
        }
    }

    /// 应用语法高亮
    pub fn highlight(&self, content: &str) -> Result<String> {
        // 这里可以集成第三方语法高亮库，如 syntect
        // 目前先返回基本的格式化
        Ok(content.to_string())
    }
}

impl FormattedOutput {
    /// 保存到文件
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<()> {
        std::fs::write(path, &self.content).map_err(SemanticDiffError::IoError)?;
        Ok(())
    }

    /// 获取内容大小（字节）
    pub fn size(&self) -> usize {
        self.content.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.content.trim().is_empty()
    }
}

/// HTML转义函数
fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests;
