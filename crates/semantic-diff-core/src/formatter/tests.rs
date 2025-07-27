//! 输出格式化器测试

use super::*;
use crate::generator::CodeSlice;
use crate::parser::{GoFunctionInfo, GoParameter, GoType};

use std::path::PathBuf;
use tempfile::tempdir;

/// 创建测试用的代码切片
fn create_test_code_slice() -> CodeSlice {
    let function = GoFunctionInfo {
        name: "TestFunction".to_string(),
        receiver: None,
        parameters: vec![GoParameter {
            name: "param1".to_string(),
            param_type: GoType {
                name: "string".to_string(),
                is_pointer: false,
                is_slice: false,
            },
        }],
        return_types: vec![GoType {
            name: "error".to_string(),
            is_pointer: false,
            is_slice: false,
        }],
        body: "func TestFunction(param1 string) error {\n    return nil\n}".to_string(),
        start_line: 10,
        end_line: 12,
        file_path: PathBuf::from("test.go"),
    };

    CodeSlice {
        header_comment: "// Test code slice\n// Generated for testing".to_string(),
        imports: vec!["import \"fmt\"".to_string()],
        type_definitions: vec!["type TestStruct struct {\n    Field string\n}".to_string()],
        function_definitions: vec![function.body.clone()],
        constants: vec!["const TestConst = \"test\"".to_string()],
        variables: vec!["var TestVar string".to_string()],
        highlighted_lines: vec![11],
        line_mapping: [(11, 5)].iter().cloned().collect(),
        line_change_types: [(11, crate::git::DiffLineType::Context)].iter().cloned().collect(),
        involved_files: vec![PathBuf::from("test.go")],
        content: "// Test code slice\n// Generated for testing\n\nimport \"fmt\"\n\ntype TestStruct struct {\n    Field string\n}\n\nconst TestConst = \"test\"\n\nvar TestVar string\n\nfunc TestFunction(param1 string) error {\n    return nil\n}".to_string(),
        dependency_graph: None,
    }
}

#[test]
fn test_formatter_config_default() {
    let config = FormatterConfig::default();

    assert_eq!(config.output_format, OutputFormat::PlainText);
    assert_eq!(config.highlight_style, HighlightStyle::Inline);
    assert!(config.show_line_numbers);
    assert!(config.show_file_paths);
    assert!(config.show_statistics);
    assert!(config.enable_colors);
    assert_eq!(config.block_title_style, BlockTitleStyle::Detailed);
    assert_eq!(config.indent_size, 4);
}

#[test]
fn test_output_renderer_creation() {
    let config = FormatterConfig::default();
    let renderer = OutputRenderer::new(config.clone());

    // 测试渲染器创建成功
    assert_eq!(renderer.config.output_format, config.output_format);
}

#[test]
fn test_render_plain_text() {
    let renderer = OutputRenderer::with_default_config();
    let code_slice = create_test_code_slice();

    let result = renderer.render(&code_slice);
    assert!(result.is_ok());

    let formatted = result.unwrap();
    assert_eq!(formatted.format, OutputFormat::PlainText);
    assert!(!formatted.content.is_empty());
    assert!(formatted.content.contains("Statistics:"));
    assert!(formatted.content.contains("Files involved:"));
    assert!(formatted.content.contains("test.go"));
}

#[test]
fn test_render_markdown() {
    let config = FormatterConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let renderer = OutputRenderer::new(config);
    let code_slice = create_test_code_slice();

    let result = renderer.render(&code_slice);
    assert!(result.is_ok());

    let formatted = result.unwrap();
    assert_eq!(formatted.format, OutputFormat::Markdown);
    assert!(formatted.content.contains("# Semantic Diff Analysis"));
    assert!(formatted.content.contains("## Statistics"));
    assert!(formatted.content.contains("## Files Involved"));
    assert!(formatted.content.contains("```go"));
}

#[test]
fn test_render_html() {
    let config = FormatterConfig {
        output_format: OutputFormat::Html,
        ..Default::default()
    };
    let renderer = OutputRenderer::new(config);
    let code_slice = create_test_code_slice();

    let result = renderer.render(&code_slice);
    assert!(result.is_ok());

    let formatted = result.unwrap();
    assert_eq!(formatted.format, OutputFormat::Html);
    assert!(formatted.content.contains("<!DOCTYPE html>"));
    assert!(
        formatted
            .content
            .contains("<title>Semantic Diff Analysis</title>")
    );
    assert!(
        formatted
            .content
            .contains("<h1>Semantic Diff Analysis</h1>")
    );
    assert!(formatted.content.contains("highlighted-line"));
}

#[test]
fn test_highlight_styles() {
    let code_slice = create_test_code_slice();

    // 测试无高亮
    let config = FormatterConfig {
        highlight_style: HighlightStyle::None,
        ..Default::default()
    };
    let renderer = OutputRenderer::new(config);
    let result = renderer.render(&code_slice).unwrap();
    assert!(!result.content.contains(">"));

    // 测试内联高亮
    let config = FormatterConfig {
        highlight_style: HighlightStyle::Inline,
        ..Default::default()
    };
    let renderer = OutputRenderer::new(config);
    let result = renderer.render(&code_slice).unwrap();
    assert!(result.content.contains(">"));

    // 测试分离式高亮
    let config = FormatterConfig {
        highlight_style: HighlightStyle::Separate,
        ..Default::default()
    };
    let renderer = OutputRenderer::new(config);
    let result = renderer.render(&code_slice).unwrap();
    assert!(result.content.contains("=== Full Content ==="));
    assert!(result.content.contains("=== Highlighted Changes ==="));
}

#[test]
fn test_line_numbers() {
    let code_slice = create_test_code_slice();

    // 测试显示行号
    let config = FormatterConfig {
        show_line_numbers: true,
        ..Default::default()
    };
    let renderer = OutputRenderer::new(config);
    let result = renderer.render(&code_slice).unwrap();
    assert!(result.content.contains("1|") || result.content.contains("   1"));

    // 测试不显示行号
    let config = FormatterConfig {
        show_line_numbers: false,
        ..Default::default()
    };
    let renderer = OutputRenderer::new(config);
    let result = renderer.render(&code_slice).unwrap();
    // 应该不包含行号格式
    assert!(!result.content.contains("1|"));
}

#[test]
fn test_statistics_display() {
    let code_slice = create_test_code_slice();

    // 测试显示统计信息
    let config = FormatterConfig {
        show_statistics: true,
        ..Default::default()
    };
    let renderer = OutputRenderer::new(config);
    let result = renderer.render(&code_slice).unwrap();
    assert!(result.content.contains("Statistics:"));
    assert!(result.content.contains("Total lines:"));

    // 测试不显示统计信息
    let config = FormatterConfig {
        show_statistics: false,
        ..Default::default()
    };
    let renderer = OutputRenderer::new(config);
    let result = renderer.render(&code_slice).unwrap();
    assert!(!result.content.contains("Statistics:"));
}

#[test]
fn test_file_paths_display() {
    let code_slice = create_test_code_slice();

    // 测试显示文件路径
    let config = FormatterConfig {
        show_file_paths: true,
        ..Default::default()
    };
    let renderer = OutputRenderer::new(config);
    let result = renderer.render(&code_slice).unwrap();
    assert!(
        result.content.contains("Files involved:") || result.content.contains("Files Involved")
    );
    assert!(result.content.contains("test.go"));

    // 测试不显示文件路径
    let config = FormatterConfig {
        show_file_paths: false,
        ..Default::default()
    };
    let renderer = OutputRenderer::new(config);
    let result = renderer.render(&code_slice).unwrap();
    assert!(!result.content.contains("Files involved:"));
}

#[test]
fn test_color_theme() {
    let theme = ColorTheme::default();

    assert_eq!(theme.added_line, "\x1b[32m");
    assert_eq!(theme.removed_line, "\x1b[31m");
    assert_eq!(theme.context_line, "\x1b[37m");
    assert_eq!(theme.line_number, "\x1b[36m");
}

#[test]
fn test_html_escape() {
    let input = "<script>alert('test');</script>";
    let escaped = html_escape(input);

    assert_eq!(
        escaped,
        "&lt;script&gt;alert(&#39;test&#39;);&lt;/script&gt;"
    );
    assert!(!escaped.contains('<'));
    assert!(!escaped.contains('>'));
    assert!(!escaped.contains('\''));
}

#[test]
fn test_formatted_output_save() {
    let renderer = OutputRenderer::with_default_config();
    let code_slice = create_test_code_slice();
    let formatted = renderer.render(&code_slice).unwrap();

    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test_output.txt");

    let result = formatted.save_to_file(&file_path);
    assert!(result.is_ok());

    // 验证文件内容
    let saved_content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(saved_content, formatted.content);
}

#[test]
fn test_formatted_output_properties() {
    let renderer = OutputRenderer::with_default_config();
    let code_slice = create_test_code_slice();
    let formatted = renderer.render(&code_slice).unwrap();

    assert!(!formatted.is_empty());
    assert!(formatted.size() > 0);
    assert_eq!(formatted.size(), formatted.content.len());
    assert!(formatted.metadata.total_lines > 0);
    assert!(formatted.metadata.content_size > 0);
    assert!(!formatted.metadata.generated_at.is_empty());
}

#[test]
fn test_syntax_highlighter() {
    let highlighter = SyntaxHighlighter::new("go".to_string(), HighlightStyle::Inline);

    let code = "func main() {\n    fmt.Println(\"Hello, World!\")\n}";
    let result = highlighter.highlight(code);

    assert!(result.is_ok());
    // 目前的实现只是返回原始内容
    assert_eq!(result.unwrap(), code);
}

#[test]
fn test_output_metadata() {
    let renderer = OutputRenderer::with_default_config();
    let code_slice = create_test_code_slice();
    let formatted = renderer.render(&code_slice).unwrap();

    let metadata = &formatted.metadata;
    assert!(metadata.total_lines > 0);
    assert_eq!(
        metadata.highlighted_lines,
        code_slice.highlighted_lines.len()
    );
    assert_eq!(metadata.files_count, code_slice.involved_files.len());
    assert!(metadata.content_size > 0);
    assert!(!metadata.generated_at.is_empty());

    // 验证时间戳格式（RFC3339）
    assert!(chrono::DateTime::parse_from_rfc3339(&metadata.generated_at).is_ok());
}

#[test]
fn test_block_title_styles() {
    // 测试不同的块标题样式
    let code_slice = create_test_code_slice();

    // 详细样式
    let config = FormatterConfig {
        block_title_style: BlockTitleStyle::Detailed,
        ..Default::default()
    };
    let renderer = OutputRenderer::new(config);
    let result = renderer.render(&code_slice).unwrap();
    assert!(result.content.contains("//"));

    // 简洁样式
    let config = FormatterConfig {
        block_title_style: BlockTitleStyle::Minimal,
        ..Default::default()
    };
    let renderer = OutputRenderer::new(config);
    let result = renderer.render(&code_slice).unwrap();
    // 当前实现中，简洁样式和详细样式相同，这里只是确保不会出错
    assert!(!result.content.is_empty());

    // 无标题
    let config = FormatterConfig {
        block_title_style: BlockTitleStyle::None,
        ..Default::default()
    };
    let renderer = OutputRenderer::new(config);
    let result = renderer.render(&code_slice).unwrap();
    // 当前实现中，无标题样式和其他样式相同，这里只是确保不会出错
    assert!(!result.content.is_empty());
}

#[test]
fn test_custom_css() {
    let config = FormatterConfig {
        output_format: OutputFormat::Html,
        custom_css: Some("body { background-color: red; }".to_string()),
        ..Default::default()
    };

    let renderer = OutputRenderer::new(config);
    let code_slice = create_test_code_slice();
    let result = renderer.render(&code_slice).unwrap();

    assert!(result.content.contains("body { background-color: red; }"));
}

#[test]
fn test_max_line_width() {
    let config = FormatterConfig {
        max_line_width: Some(80),
        ..Default::default()
    };

    let renderer = OutputRenderer::new(config);
    let code_slice = create_test_code_slice();
    let result = renderer.render(&code_slice);

    // 当前实现中，max_line_width 还没有实际使用，这里只是确保不会出错
    assert!(result.is_ok());
}

#[test]
fn test_empty_code_slice() {
    let empty_slice = CodeSlice {
        header_comment: String::new(),
        imports: vec![],
        type_definitions: vec![],
        function_definitions: vec![],
        constants: vec![],
        variables: vec![],
        highlighted_lines: vec![],
        line_mapping: std::collections::HashMap::new(),
        line_change_types: std::collections::HashMap::new(),
        involved_files: vec![],
        content: String::new(),
        dependency_graph: None,
    };

    let renderer = OutputRenderer::with_default_config();
    let result = renderer.render(&empty_slice);

    assert!(result.is_ok());
    let formatted = result.unwrap();
    // 即使是空的代码切片，也应该有一些基本的结构
    assert!(!formatted.content.is_empty()); // 至少包含统计信息等
}

#[test]
fn test_markdown_highlighting_styles() {
    let code_slice = create_test_code_slice();

    // 内联高亮
    let config = FormatterConfig {
        output_format: OutputFormat::Markdown,
        highlight_style: HighlightStyle::Inline,
        ..Default::default()
    };
    let renderer = OutputRenderer::new(config);
    let result = renderer.render(&code_slice).unwrap();
    assert!(result.content.contains("// >>>"));

    // 分离式高亮
    let config = FormatterConfig {
        output_format: OutputFormat::Markdown,
        highlight_style: HighlightStyle::Separate,
        ..Default::default()
    };
    let renderer = OutputRenderer::new(config);
    let result = renderer.render(&code_slice).unwrap();
    assert!(result.content.contains("### Full Code"));
    assert!(result.content.contains("### Highlighted Changes"));
}

#[test]
fn test_color_output_control() {
    let code_slice = create_test_code_slice();

    // 启用颜色
    let config = FormatterConfig {
        enable_colors: true,
        ..Default::default()
    };
    let renderer = OutputRenderer::new(config);
    let result = renderer.render(&code_slice).unwrap();
    assert!(result.content.contains("\x1b["));

    // 禁用颜色
    let config = FormatterConfig {
        enable_colors: false,
        ..Default::default()
    };
    let renderer = OutputRenderer::new(config);
    let result = renderer.render(&code_slice).unwrap();
    assert!(!result.content.contains("\x1b["));
}

#[test]
fn test_apply_highlighting_plain_text_inline() {
    let config = FormatterConfig {
        output_format: OutputFormat::PlainText,
        highlight_style: HighlightStyle::Inline,
        enable_colors: false, // 禁用颜色以便测试
        ..Default::default()
    };
    let renderer = OutputRenderer::new(config);

    // 创建测试用的 CodeSlice
    let mut code_slice = create_test_code_slice();
    code_slice.content = "line 1\nline 2\nline 3".to_string();
    code_slice.highlighted_lines = vec![2];
    code_slice
        .line_change_types
        .insert(2, crate::git::DiffLineType::Added);

    let result = renderer.apply_highlighting_plain_text(&code_slice);
    assert!(
        result.is_ok(),
        "apply_highlighting_plain_text should succeed"
    );

    let highlighted = result.unwrap();
    // 检查高亮功能是否正常工作（现在应该显示 + 前缀）
    assert!(
        highlighted.contains("+ line 2"),
        "Second line should be highlighted with + prefix"
    );
    assert!(highlighted.contains("line 1"), "Should contain first line");
    assert!(highlighted.contains("line 3"), "Should contain third line");
}

#[test]
fn test_apply_highlighting_plain_text_separate() {
    let config = FormatterConfig {
        output_format: OutputFormat::PlainText,
        highlight_style: HighlightStyle::Separate,
        ..Default::default()
    };
    let renderer = OutputRenderer::new(config);

    let mut code_slice = create_test_code_slice();
    code_slice.content = "line 1\nline 2\nline 3".to_string();
    code_slice.highlighted_lines = vec![2];
    code_slice
        .line_change_types
        .insert(2, crate::git::DiffLineType::Removed);

    let result = renderer.apply_highlighting_plain_text(&code_slice);
    assert!(
        result.is_ok(),
        "apply_highlighting_plain_text should succeed"
    );

    let highlighted = result.unwrap();
    assert!(
        highlighted.contains("=== Full Content ==="),
        "Should contain full content section"
    );
    assert!(
        highlighted.contains("=== Highlighted Changes ==="),
        "Should contain highlighted changes section"
    );
    assert!(
        highlighted.contains("Line 2: - line 2"),
        "Should contain highlighted line info with - prefix"
    );
}

#[test]
fn test_apply_highlighting_markdown_inline() {
    let config = FormatterConfig {
        output_format: OutputFormat::Markdown,
        highlight_style: HighlightStyle::Inline,
        ..Default::default()
    };
    let renderer = OutputRenderer::new(config);

    let mut code_slice = create_test_code_slice();
    code_slice.content = "line 1\nline 2\nline 3".to_string();
    code_slice.highlighted_lines = vec![2];
    code_slice
        .line_change_types
        .insert(2, crate::git::DiffLineType::Added);

    let result = renderer.apply_highlighting_markdown(&code_slice);
    assert!(result.is_ok(), "apply_highlighting_markdown should succeed");

    let highlighted = result.unwrap();
    assert!(
        highlighted.contains("```go"),
        "Should start with Go code block"
    );
    assert!(
        highlighted.contains("// +++ line 2"),
        "Should highlight line 2 with +++ comment"
    );
    assert!(
        highlighted.contains("line 1\n"),
        "Should contain unchanged line 1"
    );
    assert!(
        highlighted.contains("line 3"),
        "Should contain unchanged line 3"
    );
}

#[test]
fn test_apply_highlighting_html() {
    let config = FormatterConfig {
        output_format: OutputFormat::Html,
        highlight_style: HighlightStyle::Inline,
        ..Default::default()
    };
    let renderer = OutputRenderer::new(config);

    let mut code_slice = create_test_code_slice();
    code_slice.content = "line 1\nline 2\nline 3".to_string();
    code_slice.highlighted_lines = vec![2];
    code_slice
        .line_change_types
        .insert(2, crate::git::DiffLineType::Added);

    let result = renderer.apply_highlighting_html(&code_slice);
    assert!(result.is_ok(), "apply_highlighting_html should succeed");

    let highlighted = result.unwrap();
    assert!(
        highlighted.contains("<pre><code class=\"language-go\">"),
        "Should start with HTML code block"
    );
    assert!(
        highlighted.contains("added-line"),
        "Should have added-line class"
    );
    assert!(
        highlighted.contains("change-prefix"),
        "Should have change prefix span"
    );
}

#[test]
fn test_highlighting_with_no_highlighted_lines() {
    let config = FormatterConfig {
        output_format: OutputFormat::PlainText,
        highlight_style: HighlightStyle::Inline,
        ..Default::default()
    };
    let renderer = OutputRenderer::new(config);

    let mut code_slice = create_test_code_slice();
    code_slice.content = "line 1\nline 2\nline 3".to_string();
    code_slice.highlighted_lines = vec![]; // 没有高亮行

    let result = renderer.apply_highlighting_plain_text(&code_slice);
    assert!(result.is_ok(), "Should handle empty highlighted lines");

    let highlighted = result.unwrap();
    assert!(
        !highlighted.contains("+") && !highlighted.contains("-"),
        "Should not contain change markers"
    );
}

#[test]
fn test_highlighting_with_colors_disabled() {
    let config = FormatterConfig {
        output_format: OutputFormat::PlainText,
        highlight_style: HighlightStyle::Inline,
        enable_colors: false,
        ..Default::default()
    };
    let renderer = OutputRenderer::new(config);

    let mut code_slice = create_test_code_slice();
    code_slice.content = "line 1\nline 2\nline 3".to_string();
    code_slice.highlighted_lines = vec![2];
    code_slice
        .line_change_types
        .insert(2, crate::git::DiffLineType::Removed);

    let result = renderer.apply_highlighting_plain_text(&code_slice);
    assert!(result.is_ok(), "Should work with colors disabled");

    let highlighted = result.unwrap();
    assert!(
        !highlighted.contains("\x1b["),
        "Should not contain ANSI color codes"
    );
    assert!(
        highlighted.contains("- line 2"),
        "Should contain - prefix for removed line"
    );
}
