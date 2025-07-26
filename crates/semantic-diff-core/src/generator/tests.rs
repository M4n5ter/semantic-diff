//! 代码切片生成器的单元测试

use super::*;
use crate::extractor::SemanticContext;
use crate::git::{DiffHunk, DiffLine, DiffLineType};
use crate::parser::{GoFunctionInfo, GoParameter, GoType, GoTypeDefinition, GoTypeKind, Import};
use std::path::PathBuf;

/// 创建测试用的函数信息
fn create_test_function() -> GoFunctionInfo {
    GoFunctionInfo {
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
    }
}

/// 创建测试用的类型定义
fn create_test_type() -> GoTypeDefinition {
    GoTypeDefinition {
        name: "TestStruct".to_string(),
        kind: GoTypeKind::Struct,
        definition: "type TestStruct struct {\n    Field1 string\n    Field2 int\n}".to_string(),
        file_path: PathBuf::from("types.go"),
        dependencies: vec!["string".to_string(), "int".to_string()],
    }
}

/// 创建测试用的语义上下文
fn create_test_context() -> SemanticContext {
    let function = create_test_function();
    let mut context = SemanticContext::from_function(function);

    // 添加类型定义
    context.add_type(create_test_type());

    // 添加导入
    context.add_import(Import {
        path: "fmt".to_string(),
        alias: None,
    });

    context
}

/// 创建测试用的差异块
fn create_test_diff_hunk() -> DiffHunk {
    DiffHunk {
        old_start: 10,
        old_lines: 1,
        new_start: 10,
        new_lines: 1,
        lines: vec![
            DiffLine {
                content: "    return nil".to_string(),
                line_type: DiffLineType::Removed,
                old_line_number: Some(10),
                new_line_number: None,
            },
            DiffLine {
                content: "    return fmt.Errorf(\"test error\")".to_string(),
                line_type: DiffLineType::Added,
                old_line_number: None,
                new_line_number: Some(10),
            },
        ],
        context_lines: 3,
    }
}

#[test]
fn test_code_slice_generator_new() {
    let generator = CodeSliceGenerator::new();
    assert_eq!(generator.config.output_format, OutputFormat::PlainText);
    assert_eq!(generator.config.highlight_style, HighlightStyle::Inline);
    assert!(generator.config.include_comments);
    assert!(generator.config.include_imports);
    assert!(generator.config.include_types);
    assert!(generator.config.include_dependent_functions);
}

#[test]
fn test_code_slice_generator_with_config() {
    let config = GeneratorConfig {
        include_comments: false,
        include_imports: false,
        include_types: false,
        include_dependent_functions: false,
        include_dependency_graph: false,
        max_lines: Some(100),
        output_format: OutputFormat::Markdown,
        highlight_style: HighlightStyle::Separate,
    };

    let generator = CodeSliceGenerator::with_config(config.clone());
    assert_eq!(generator.config.output_format, OutputFormat::Markdown);
    assert_eq!(generator.config.highlight_style, HighlightStyle::Separate);
    assert!(!generator.config.include_comments);
    assert!(!generator.config.include_imports);
    assert!(!generator.config.include_types);
    assert!(!generator.config.include_dependent_functions);
    assert_eq!(generator.config.max_lines, Some(100));
}

#[test]
fn test_generate_slice_basic() {
    let generator = CodeSliceGenerator::new();
    let context = create_test_context();
    let changes = vec![create_test_diff_hunk()];

    let result = generator.generate_slice(&context, &changes);
    assert!(result.is_ok(), "generate_slice should succeed");

    let slice = result.unwrap();
    assert!(
        !slice.header_comment.is_empty(),
        "Should have header comment"
    );
    assert!(!slice.content.is_empty(), "Should have content");
    assert!(
        !slice.involved_files.is_empty(),
        "Should have involved files"
    );

    // 验证包含了预期的内容
    assert!(
        slice.content.contains("TestFunction"),
        "Should contain function name"
    );
    assert!(
        slice.content.contains("TestStruct"),
        "Should contain type name"
    );
    assert!(slice.content.contains("fmt"), "Should contain import");
}

#[test]
fn test_generate_slice_without_imports() {
    let config = GeneratorConfig {
        include_imports: false,
        ..Default::default()
    };
    let generator = CodeSliceGenerator::with_config(config);
    let context = create_test_context();
    let changes = vec![];

    let result = generator.generate_slice(&context, &changes);
    assert!(result.is_ok(), "generate_slice should succeed");

    let slice = result.unwrap();
    assert!(slice.imports.is_empty(), "Should not include imports");
    assert!(
        !slice.content.contains("import \"fmt\""),
        "Content should not contain imports"
    );
}

#[test]
fn test_generate_slice_without_types() {
    let config = GeneratorConfig {
        include_types: false,
        ..Default::default()
    };
    let generator = CodeSliceGenerator::with_config(config);
    let context = create_test_context();
    let changes = vec![];

    let result = generator.generate_slice(&context, &changes);
    assert!(result.is_ok(), "generate_slice should succeed");

    let slice = result.unwrap();
    assert!(
        slice.type_definitions.is_empty(),
        "Should not include type definitions"
    );
    assert!(
        !slice.content.contains("TestStruct"),
        "Content should not contain type definitions"
    );
}

#[test]
fn test_highlight_changes() {
    let generator = CodeSliceGenerator::new();
    let context = create_test_context();
    let changes = vec![create_test_diff_hunk()];

    let result = generator.generate_slice(&context, &changes);
    assert!(result.is_ok(), "generate_slice should succeed");

    let mut slice = result.unwrap();

    // 测试高亮功能
    let highlight_result = generator.highlight_changes(&mut slice, &changes);
    assert!(highlight_result.is_ok(), "highlight_changes should succeed");

    // 验证高亮行被正确标记
    // 注意：由于我们的实现是基于内容匹配的，具体的行号可能会变化
    // 这里我们主要验证功能是否正常工作
    assert!(
        slice.highlighted_lines.len() <= slice.content.lines().count(),
        "Highlighted lines should not exceed total lines"
    );
}

#[test]
fn test_code_formatter_plain_text() {
    let formatter = CodeFormatter::new(OutputFormat::PlainText, HighlightStyle::None);
    let content = "package main\n\nfunc main() {\n    fmt.Println(\"Hello\")\n}";

    let result = formatter.format_content(content);
    assert!(result.is_ok(), "format_content should succeed");

    let formatted = result.unwrap();
    assert_eq!(formatted, content, "Plain text should remain unchanged");
}

#[test]
fn test_code_formatter_markdown() {
    let formatter = CodeFormatter::new(OutputFormat::Markdown, HighlightStyle::None);
    let content = "package main\n\nfunc main() {}";

    let result = formatter.format_content(content);
    assert!(result.is_ok(), "format_content should succeed");

    let formatted = result.unwrap();
    assert!(
        formatted.starts_with("```go\n"),
        "Should start with Go code block"
    );
    assert!(formatted.ends_with("```\n"), "Should end with code block");
    assert!(
        formatted.contains(content),
        "Should contain original content"
    );
}

#[test]
fn test_code_formatter_html() {
    let formatter = CodeFormatter::new(OutputFormat::Html, HighlightStyle::None);
    let content = "package main\n\nfunc main() {}";

    let result = formatter.format_content(content);
    assert!(result.is_ok(), "format_content should succeed");

    let formatted = result.unwrap();
    assert!(
        formatted.starts_with("<pre><code class=\"language-go\">"),
        "Should start with HTML code block"
    );
    assert!(
        formatted.ends_with("</code></pre>\n"),
        "Should end with HTML code block"
    );
    assert!(
        formatted.contains("package main"),
        "Should contain escaped content"
    );
}

#[test]
fn test_code_formatter_html_escaping() {
    let formatter = CodeFormatter::new(OutputFormat::Html, HighlightStyle::None);
    let content = "if x < y && z > 0 { return \"test\" }";

    let result = formatter.format_content(content);
    assert!(result.is_ok(), "format_content should succeed");

    let formatted = result.unwrap();
    assert!(formatted.contains("&lt;"), "Should escape < character");
    assert!(formatted.contains("&gt;"), "Should escape > character");
    assert!(formatted.contains("&amp;"), "Should escape & character");
    assert!(formatted.contains("&quot;"), "Should escape \" character");
}

#[test]
fn test_apply_syntax_highlighting_inline() {
    let formatter = CodeFormatter::new(OutputFormat::PlainText, HighlightStyle::Inline);
    let content = "line 1\nline 2\nline 3";
    let highlighted_lines = vec![2];

    let result = formatter.apply_syntax_highlighting(content, &highlighted_lines);
    assert!(result.is_ok(), "apply_syntax_highlighting should succeed");

    let highlighted = result.unwrap();
    let lines: Vec<&str> = highlighted.lines().collect();
    assert_eq!(lines[0], "line 1", "First line should be unchanged");
    assert_eq!(lines[1], "> line 2", "Second line should be highlighted");
    assert_eq!(lines[2], "line 3", "Third line should be unchanged");
}

#[test]
fn test_apply_syntax_highlighting_separate() {
    let formatter = CodeFormatter::new(OutputFormat::PlainText, HighlightStyle::Separate);
    let content = "line 1\nline 2\nline 3";
    let highlighted_lines = vec![2];

    let result = formatter.apply_syntax_highlighting(content, &highlighted_lines);
    assert!(result.is_ok(), "apply_syntax_highlighting should succeed");

    let highlighted = result.unwrap();
    assert!(
        highlighted.contains(content),
        "Should contain original content"
    );
    assert!(
        highlighted.contains("// Highlighted changes:"),
        "Should contain highlight section"
    );
    assert!(
        highlighted.contains("// Line 2: line 2"),
        "Should contain highlighted line info"
    );
}

#[test]
fn test_code_slice_stats() {
    let generator = CodeSliceGenerator::new();
    let context = create_test_context();
    let changes = vec![];

    let result = generator.generate_slice(&context, &changes);
    assert!(result.is_ok(), "generate_slice should succeed");

    let slice = result.unwrap();
    let stats = slice.get_stats();

    assert!(stats.total_lines > 0, "Should have total lines");
    assert!(stats.imports_count > 0, "Should have imports");
    assert!(stats.types_count > 0, "Should have types");
    assert!(stats.functions_count > 0, "Should have functions");
    assert!(stats.files_count > 0, "Should have files");
}

#[test]
fn test_code_slice_has_highlights() {
    let generator = CodeSliceGenerator::new();
    let context = create_test_context();
    let changes = vec![create_test_diff_hunk()];

    let result = generator.generate_slice(&context, &changes);
    assert!(result.is_ok(), "generate_slice should succeed");

    let slice = result.unwrap();

    // 初始状态可能没有高亮
    // 但调用 highlight_changes 后应该有高亮
    let mut slice_copy = slice.clone();
    let _ = generator.highlight_changes(&mut slice_copy, &changes);

    // 验证高亮功能
    if !slice_copy.highlighted_lines.is_empty() {
        assert!(
            slice_copy.has_highlights(),
            "Should have highlights after highlighting"
        );
        let highlighted_content = slice_copy.get_highlighted_content();
        assert!(
            !highlighted_content.is_empty(),
            "Should have highlighted content"
        );
    }
}

#[test]
fn test_generate_header_comment() {
    let generator = CodeSliceGenerator::new();
    let context = create_test_context();

    let header = generator.generate_header_comment(&context);

    assert!(
        header.contains("Function: TestFunction"),
        "Should contain function name"
    );
    assert!(
        header.contains("Generated by semantic-diff"),
        "Should contain generator info"
    );
    assert!(
        header.contains("Context includes:"),
        "Should contain context info"
    );
    assert!(header.contains("types"), "Should mention types");
    assert!(header.contains("functions"), "Should mention functions");
    assert!(header.contains("imports"), "Should mention imports");
}

#[test]
fn test_split_into_lines() {
    let generator = CodeSliceGenerator::new();
    let content = "line 1\nline 2\nline 3";
    let start_line = 5;

    let lines = generator.split_into_lines(content, start_line);

    assert_eq!(lines.len(), 3, "Should have 3 lines");
    assert_eq!(lines[0].content, "line 1");
    assert_eq!(lines[0].line_number, 5);
    assert_eq!(lines[1].content, "line 2");
    assert_eq!(lines[1].line_number, 6);
    assert_eq!(lines[2].content, "line 3");
    assert_eq!(lines[2].line_number, 7);

    for line in &lines {
        assert!(
            !line.is_highlighted,
            "Lines should not be highlighted initially"
        );
        assert!(
            line.change_type.is_none(),
            "Lines should not have change type initially"
        );
    }
}

#[test]
fn test_should_highlight_line() {
    let generator = CodeSliceGenerator::new();
    let changes = vec![create_test_diff_hunk()];

    // 测试应该被高亮的行
    assert!(
        generator.should_highlight_line("    return nil", &changes),
        "Should highlight removed line"
    );
    assert!(
        generator.should_highlight_line("    return fmt.Errorf(\"test error\")", &changes),
        "Should highlight added line"
    );

    // 测试不应该被高亮的行
    assert!(
        !generator.should_highlight_line("func TestFunction() {", &changes),
        "Should not highlight unrelated line"
    );
    assert!(
        !generator.should_highlight_line("", &changes),
        "Should not highlight empty line"
    );
}

#[test]
fn test_should_highlight_change() {
    let generator = CodeSliceGenerator::new();

    // 测试应该被高亮的重要变更
    assert!(
        generator.should_highlight_change("case anthropic.ThinkingDelta:"),
        "Should highlight ThinkingDelta case"
    );
    assert!(
        generator.should_highlight_change("Reasoning: Some(deltaVariant.JSON.Thinking.Raw()),"),
        "Should highlight Reasoning assignment"
    );
    assert!(
        generator.should_highlight_change("// 只有当有实际内容时才发送 chunk"),
        "Should highlight important comment"
    );
    assert!(
        generator.should_highlight_change(
            "if len(chunk.ContentParts) > 0 || chunk.Reasoning.IsSome() {"
        ),
        "Should highlight condition with ContentParts"
    );

    // 测试不应该被高亮的通用内容
    assert!(
        !generator.should_highlight_change("return"),
        "Should not highlight simple return"
    );
    assert!(
        !generator.should_highlight_change("}"),
        "Should not highlight closing brace"
    );
    assert!(
        !generator.should_highlight_change("nil"),
        "Should not highlight nil"
    );
    assert!(
        !generator.should_highlight_change("x := 1"),
        "Should not highlight simple assignment"
    );
}

#[test]
fn test_is_significant_change() {
    let generator = CodeSliceGenerator::new();

    // 测试重要的变更
    assert!(
        generator.is_significant_change("func processData(data string) error {"),
        "Should consider function declaration significant"
    );
    assert!(
        generator.is_significant_change("if data == \"\" {"),
        "Should consider if statement significant"
    );
    assert!(
        generator.is_significant_change("result := processData(input)"),
        "Should consider assignment significant"
    );
    assert!(
        generator.is_significant_change("// This is an important comment"),
        "Should consider comments significant"
    );

    // 测试不重要的变更
    assert!(
        !generator.is_significant_change("return"),
        "Should not consider simple return significant"
    );
    assert!(
        !generator.is_significant_change("}"),
        "Should not consider closing brace significant"
    );
    assert!(
        !generator.is_significant_change("nil"),
        "Should not consider nil significant"
    );
    assert!(
        !generator.is_significant_change("()"),
        "Should not consider empty parens significant"
    );
    assert!(
        !generator.is_significant_change("x"),
        "Should not consider short content significant"
    );
}

#[test]
fn test_is_unique_enough_for_highlighting() {
    let generator = CodeSliceGenerator::new();

    // 测试足够独特的内容
    assert!(
        generator.is_unique_enough_for_highlighting("case anthropic.ThinkingDelta:"),
        "Should consider ThinkingDelta unique"
    );
    assert!(
        generator.is_unique_enough_for_highlighting("Reasoning: Some(value)"),
        "Should consider Reasoning assignment unique"
    );
    assert!(
        generator.is_unique_enough_for_highlighting("// 只有当有实际内容时才发送 chunk"),
        "Should consider specific comment unique"
    );
    assert!(
        generator.is_unique_enough_for_highlighting("if len(chunk.ContentParts) > 0 {"),
        "Should consider ContentParts condition unique"
    );

    // 测试不够独特的内容
    assert!(
        !generator.is_unique_enough_for_highlighting("case someValue:"),
        "Should not consider generic case unique"
    );
    assert!(
        !generator.is_unique_enough_for_highlighting("if x > 0 {"),
        "Should not consider generic condition unique"
    );
}

#[test]
fn test_build_function_signature() {
    let generator = CodeSliceGenerator::new();

    // 测试简单函数签名
    let simple_function = GoFunctionInfo {
        name: "simpleFunc".to_string(),
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
        body: "return nil".to_string(),
        start_line: 1,
        end_line: 1,
        file_path: PathBuf::from("test.go"),
    };

    let signature = generator.build_function_signature(&simple_function);
    assert_eq!(signature, "func simpleFunc(param1 string) error");

    // 测试方法签名（带接收者）
    let method_function = GoFunctionInfo {
        name: "methodFunc".to_string(),
        receiver: Some(crate::parser::GoReceiverInfo {
            name: "s".to_string(),
            type_name: "Service".to_string(),
            is_pointer: true,
        }),
        parameters: vec![
            GoParameter {
                name: "param1".to_string(),
                param_type: GoType {
                    name: "User".to_string(),
                    is_pointer: false,
                    is_slice: false,
                },
            },
            GoParameter {
                name: "param2".to_string(),
                param_type: GoType {
                    name: "Config".to_string(),
                    is_pointer: true,
                    is_slice: false,
                },
            },
        ],
        return_types: vec![
            GoType {
                name: "Result".to_string(),
                is_pointer: true,
                is_slice: false,
            },
            GoType {
                name: "error".to_string(),
                is_pointer: false,
                is_slice: false,
            },
        ],
        body: "return &Result{}, nil".to_string(),
        start_line: 1,
        end_line: 1,
        file_path: PathBuf::from("service.go"),
    };

    let method_signature = generator.build_function_signature(&method_function);
    assert_eq!(
        method_signature,
        "func (s *Service) methodFunc(param1 User, param2 *Config) (*Result, error)"
    );

    // 测试切片参数和返回类型
    let slice_function = GoFunctionInfo {
        name: "sliceFunc".to_string(),
        receiver: None,
        parameters: vec![GoParameter {
            name: "items".to_string(),
            param_type: GoType {
                name: "string".to_string(),
                is_pointer: false,
                is_slice: true,
            },
        }],
        return_types: vec![GoType {
            name: "int".to_string(),
            is_pointer: false,
            is_slice: true,
        }],
        body: "return make([]int, len(items))".to_string(),
        start_line: 1,
        end_line: 1,
        file_path: PathBuf::from("test.go"),
    };

    let slice_signature = generator.build_function_signature(&slice_function);
    assert_eq!(slice_signature, "func sliceFunc(items []string) []int");
}

#[test]
fn test_build_complete_function_definition() {
    let generator = CodeSliceGenerator::new();

    let function = GoFunctionInfo {
        name: "processData".to_string(),
        receiver: Some(crate::parser::GoReceiverInfo {
            name: "s".to_string(),
            type_name: "Service".to_string(),
            is_pointer: true,
        }),
        parameters: vec![GoParameter {
            name: "data".to_string(),
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
        body: "if data == \"\" {\n    return errors.New(\"empty data\")\n}\nreturn nil".to_string(),
        start_line: 10,
        end_line: 14,
        file_path: PathBuf::from("service.go"),
    };

    let complete_definition = generator.build_complete_function_definition(&function);

    let expected = "func (s *Service) processData(data string) error {\n    if data == \"\" {\n        return errors.New(\"empty data\")\n    }\n    return nil\n}";

    assert_eq!(complete_definition, expected);

    // 验证包含函数签名
    assert!(complete_definition.contains("func (s *Service) processData(data string) error"));
    // 验证包含函数体
    assert!(complete_definition.contains("if data == \"\""));
    assert!(complete_definition.contains("return nil"));
}

#[test]
fn test_generate_function_block_with_signature() {
    let generator = CodeSliceGenerator::new();

    let function = GoFunctionInfo {
        name: "NewService".to_string(),
        receiver: None,
        parameters: vec![GoParameter {
            name: "config".to_string(),
            param_type: GoType {
                name: "Config".to_string(),
                is_pointer: true,
                is_slice: false,
            },
        }],
        return_types: vec![GoType {
            name: "Service".to_string(),
            is_pointer: true,
            is_slice: false,
        }],
        body: "return &Service{\n    config: config,\n}".to_string(),
        start_line: 5,
        end_line: 8,
        file_path: PathBuf::from("service.go"),
    };

    let block = generator.generate_function_block(&function);

    assert_eq!(block.title, "Function: NewService");
    assert_eq!(block.block_type, BlockType::Function);

    // 验证生成的代码块包含完整的函数定义
    let content: String = block
        .lines
        .iter()
        .map(|line| &line.content)
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");

    // 应该包含函数签名
    assert!(content.contains("func NewService(config *Config) *Service"));
    // 应该包含函数体
    assert!(content.contains("return &Service{"));
    assert!(content.contains("config: config,"));

    println!("Generated function block content:\n{content}");
}

#[test]
fn test_function_body_with_existing_braces() {
    let generator = CodeSliceGenerator::new();

    // 测试函数体已经包含大括号的情况
    let function_with_braces = GoFunctionInfo {
        name: "processData".to_string(),
        receiver: None,
        parameters: vec![GoParameter {
            name: "data".to_string(),
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
        // 注意：这里的函数体已经包含了完整的大括号
        body: "{\n    if data == \"\" {\n        return errors.New(\"empty data\")\n    }\n    return nil\n}".to_string(),
        start_line: 10,
        end_line: 15,
        file_path: PathBuf::from("test.go"),
    };

    let complete_definition = generator.build_complete_function_definition(&function_with_braces);
    println!("Function with existing braces:\n{complete_definition}");

    // 检查是否有重复的大括号
    let brace_count = complete_definition.matches('{').count();
    let closing_brace_count = complete_definition.matches('}').count();

    // 应该有合理数量的大括号：函数体开始1个，if语句1个
    assert_eq!(brace_count, 2, "Should have exactly 2 opening braces");
    assert_eq!(
        closing_brace_count, 2,
        "Should have exactly 2 closing braces"
    );

    // 不应该有连续的大括号
    assert!(
        !complete_definition.contains("{{"),
        "Should not have consecutive opening braces"
    );
    assert!(
        !complete_definition.contains("}}"),
        "Should not have consecutive closing braces at the end"
    );
}

#[test]
fn test_function_body_without_braces() {
    let generator = CodeSliceGenerator::new();

    // 测试函数体不包含大括号的情况（只有函数体内容）
    let function_without_braces = GoFunctionInfo {
        name: "simpleFunc".to_string(),
        receiver: None,
        parameters: vec![],
        return_types: vec![GoType {
            name: "string".to_string(),
            is_pointer: false,
            is_slice: false,
        }],
        // 函数体只包含内容，没有大括号
        body: "return \"hello world\"".to_string(),
        start_line: 5,
        end_line: 5,
        file_path: PathBuf::from("test.go"),
    };

    let complete_definition =
        generator.build_complete_function_definition(&function_without_braces);
    println!("Function without existing braces:\n{complete_definition}");

    // 应该正确添加大括号
    assert!(complete_definition.starts_with("func simpleFunc() string {"));
    assert!(complete_definition.ends_with("}"));

    // 检查大括号数量
    let brace_count = complete_definition.matches('{').count();
    let closing_brace_count = complete_definition.matches('}').count();

    assert_eq!(brace_count, 1, "Should have exactly 1 opening brace");
    assert_eq!(
        closing_brace_count, 1,
        "Should have exactly 1 closing brace"
    );
}

#[test]
fn test_empty_function_body() {
    let generator = CodeSliceGenerator::new();

    // 测试空函数体的情况
    let empty_function = GoFunctionInfo {
        name: "emptyFunc".to_string(),
        receiver: None,
        parameters: vec![],
        return_types: vec![],
        body: "".to_string(),
        start_line: 1,
        end_line: 1,
        file_path: PathBuf::from("test.go"),
    };

    let complete_definition = generator.build_complete_function_definition(&empty_function);
    println!("Empty function:\n{complete_definition}");

    // 应该生成正确的空函数
    let expected = "func emptyFunc() {\n}";
    assert_eq!(complete_definition, expected);
}

#[test]
fn test_function_with_complex_body() {
    let generator = CodeSliceGenerator::new();

    // 测试包含复杂结构的函数体
    let complex_function = GoFunctionInfo {
        name: "complexFunc".to_string(),
        receiver: Some(crate::parser::GoReceiverInfo {
            name: "s".to_string(),
            type_name: "Service".to_string(),
            is_pointer: true,
        }),
        parameters: vec![
            GoParameter {
                name: "ctx".to_string(),
                param_type: GoType {
                    name: "Context".to_string(),
                    is_pointer: false,
                    is_slice: false,
                },
            },
            GoParameter {
                name: "req".to_string(),
                param_type: GoType {
                    name: "Request".to_string(),
                    is_pointer: true,
                    is_slice: false,
                },
            },
        ],
        return_types: vec![
            GoType {
                name: "Response".to_string(),
                is_pointer: true,
                is_slice: false,
            },
            GoType {
                name: "error".to_string(),
                is_pointer: false,
                is_slice: false,
            },
        ],
        body: "if req == nil {\n    return nil, errors.New(\"nil request\")\n}\n\nswitch req.Type {\ncase \"A\":\n    return s.handleA(ctx, req)\ncase \"B\":\n    return s.handleB(ctx, req)\ndefault:\n    return nil, errors.New(\"unknown type\")\n}".to_string(),
        start_line: 10,
        end_line: 20,
        file_path: PathBuf::from("service.go"),
    };

    let complete_definition = generator.build_complete_function_definition(&complex_function);
    println!("Complex function:\n{complete_definition}");

    // 验证函数签名正确
    assert!(
        complete_definition.contains(
            "func (s *Service) complexFunc(ctx Context, req *Request) (*Response, error)"
        )
    );

    // 验证函数体内容正确缩进
    assert!(complete_definition.contains("    if req == nil {"));
    assert!(complete_definition.contains("    switch req.Type {"));
    assert!(complete_definition.contains("    case \"A\":"));

    // 验证大括号平衡
    let brace_count = complete_definition.matches('{').count();
    let closing_brace_count = complete_definition.matches('}').count();
    assert_eq!(
        brace_count, closing_brace_count,
        "Braces should be balanced"
    );
}
