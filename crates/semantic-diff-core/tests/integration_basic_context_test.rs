//! 基础语义上下文提取器集成测试
//!
//! 测试基本功能，确保核心API工作正常

use semantic_diff_core::extractor::{ChangeTarget, SemanticContextExtractor};
use semantic_diff_core::parser::common::LanguageParser;
use semantic_diff_core::parser::{
    GoConstantDefinition, GoDeclaration, GoFunctionInfo, GoLanguageInfo, GoParameter, GoType,
    GoTypeDefinition, GoTypeKind, GoVariableDefinition, Import, SourceFile, SupportedLanguage,
};
use std::path::PathBuf;

/// 创建简单的测试源文件
fn create_simple_test_source_file() -> SourceFile {
    let mut parser =
        semantic_diff_core::parser::go::GoParser::new().expect("Failed to create parser");
    let source_code = r#"package main

import "fmt"

type User struct {
    Name string
    Age  int
}

const DefaultName = "Anonymous"

var GlobalUser User

func CreateUser(name string, age int) *User {
    return &User{Name: name, Age: age}
}

func (u *User) GetName() string {
    return u.Name
}
"#;

    let syntax_tree = parser
        .parse_source(source_code)
        .expect("Failed to parse source");
    let mut go_info = GoLanguageInfo::new("main".to_string());

    // 添加导入
    go_info.add_import(Import {
        path: "fmt".to_string(),
        alias: None,
    });

    // 添加类型定义
    let user_type = GoTypeDefinition {
        name: "User".to_string(),
        kind: GoTypeKind::Struct,
        definition: "type User struct { Name string; Age int }".to_string(),
        file_path: PathBuf::from("main.go"),
        dependencies: vec![],
    };
    go_info.add_go_declaration(GoDeclaration::Type(user_type));

    // 添加常量
    let default_name_const = GoConstantDefinition {
        name: "DefaultName".to_string(),
        value: "\"Anonymous\"".to_string(),
        const_type: Some(GoType {
            name: "string".to_string(),
            is_pointer: false,
            is_slice: false,
        }),
        start_line: 9,
        end_line: 9,
        file_path: PathBuf::from("main.go"),
    };
    go_info.add_go_declaration(GoDeclaration::Constant(default_name_const));

    // 添加变量
    let global_user_var = GoVariableDefinition {
        name: "GlobalUser".to_string(),
        var_type: Some(GoType {
            name: "User".to_string(),
            is_pointer: false,
            is_slice: false,
        }),
        initial_value: None,
        start_line: 11,
        end_line: 11,
        file_path: PathBuf::from("main.go"),
    };
    go_info.add_go_declaration(GoDeclaration::Variable(global_user_var));

    // 添加函数
    let create_user_func = GoFunctionInfo {
        name: "CreateUser".to_string(),
        receiver: None,
        parameters: vec![
            GoParameter {
                name: "name".to_string(),
                param_type: GoType {
                    name: "string".to_string(),
                    is_pointer: false,
                    is_slice: false,
                },
            },
            GoParameter {
                name: "age".to_string(),
                param_type: GoType {
                    name: "int".to_string(),
                    is_pointer: false,
                    is_slice: false,
                },
            },
        ],
        return_types: vec![GoType {
            name: "User".to_string(),
            is_pointer: true,
            is_slice: false,
        }],
        body: "return &User{Name: name, Age: age}".to_string(),
        start_line: 13,
        end_line: 15,
        file_path: PathBuf::from("main.go"),
    };
    go_info.add_go_declaration(GoDeclaration::Function(create_user_func));

    // 添加方法
    let get_name_method = GoFunctionInfo {
        name: "GetName".to_string(),
        receiver: Some(semantic_diff_core::parser::GoReceiverInfo {
            name: "u".to_string(),
            type_name: "User".to_string(),
            is_pointer: true,
        }),
        parameters: vec![],
        return_types: vec![GoType {
            name: "string".to_string(),
            is_pointer: false,
            is_slice: false,
        }],
        body: "return u.Name".to_string(),
        start_line: 17,
        end_line: 19,
        file_path: PathBuf::from("main.go"),
    };
    go_info.add_go_declaration(GoDeclaration::Method(get_name_method));

    SourceFile {
        path: PathBuf::from("main.go"),
        source_code: source_code.to_string(),
        syntax_tree,
        language: SupportedLanguage::Go,
        language_specific: Box::new(go_info),
    }
}

#[test]
fn test_basic_function_context_extraction() {
    let source_file = create_simple_test_source_file();
    let extractor = SemanticContextExtractor::new();

    // 测试提取 CreateUser 函数的上下文
    let create_user_func = GoFunctionInfo {
        name: "CreateUser".to_string(),
        receiver: None,
        parameters: vec![
            GoParameter {
                name: "name".to_string(),
                param_type: GoType {
                    name: "string".to_string(),
                    is_pointer: false,
                    is_slice: false,
                },
            },
            GoParameter {
                name: "age".to_string(),
                param_type: GoType {
                    name: "int".to_string(),
                    is_pointer: false,
                    is_slice: false,
                },
            },
        ],
        return_types: vec![GoType {
            name: "User".to_string(),
            is_pointer: true,
            is_slice: false,
        }],
        body: "return &User{Name: name, Age: age}".to_string(),
        start_line: 13,
        end_line: 15,
        file_path: PathBuf::from("main.go"),
    };

    let context = extractor
        .extract_context(&create_user_func, &[source_file])
        .expect("Failed to extract context");

    // 验证基本功能
    assert_eq!(context.change_target.name(), "CreateUser");
    assert_eq!(
        context.change_target.change_type(),
        semantic_diff_core::extractor::ChangeType::Function
    );

    // 验证统计信息
    let stats = context.get_stats();
    println!("Function context stats: {stats:?}");

    // 验证上下文不为空
    assert!(!context.is_empty());

    // 验证涉及的文件
    let involved_files = context.get_involved_files();
    assert!(!involved_files.is_empty());
    assert!(involved_files.contains(&PathBuf::from("main.go")));
}

#[test]
fn test_basic_type_context_extraction() {
    let source_file = create_simple_test_source_file();
    let extractor = SemanticContextExtractor::new();

    // 测试提取 User 类型的上下文
    let user_type = GoTypeDefinition {
        name: "User".to_string(),
        kind: GoTypeKind::Struct,
        definition: "type User struct { Name string; Age int }".to_string(),
        file_path: PathBuf::from("main.go"),
        dependencies: vec![],
    };

    let change_target = ChangeTarget::Type(user_type);
    let context = extractor
        .extract_context_for_target(change_target, &[source_file])
        .expect("Failed to extract type context");

    // 验证基本功能
    assert_eq!(context.change_target.name(), "User");
    assert_eq!(
        context.change_target.change_type(),
        semantic_diff_core::extractor::ChangeType::Type
    );

    // 验证统计信息
    let stats = context.get_stats();
    println!("Type context stats: {stats:?}");

    // 验证涉及的文件
    let involved_files = context.get_involved_files();
    assert!(!involved_files.is_empty());
}

#[test]
fn test_basic_variable_context_extraction() {
    let source_file = create_simple_test_source_file();
    let extractor = SemanticContextExtractor::new();

    // 测试提取 GlobalUser 变量的上下文
    let global_user_var = GoVariableDefinition {
        name: "GlobalUser".to_string(),
        var_type: Some(GoType {
            name: "User".to_string(),
            is_pointer: false,
            is_slice: false,
        }),
        initial_value: None,
        start_line: 11,
        end_line: 11,
        file_path: PathBuf::from("main.go"),
    };

    let change_target = ChangeTarget::Variable(global_user_var);
    let context = extractor
        .extract_context_for_target(change_target, &[source_file])
        .expect("Failed to extract variable context");

    // 验证基本功能
    assert_eq!(context.change_target.name(), "GlobalUser");
    assert_eq!(
        context.change_target.change_type(),
        semantic_diff_core::extractor::ChangeType::Variable
    );

    // 验证统计信息
    let stats = context.get_stats();
    println!("Variable context stats: {stats:?}");
}

#[test]
fn test_basic_constant_context_extraction() {
    let source_file = create_simple_test_source_file();
    let extractor = SemanticContextExtractor::new();

    // 测试提取 DefaultName 常量的上下文
    let default_name_const = GoConstantDefinition {
        name: "DefaultName".to_string(),
        value: "\"Anonymous\"".to_string(),
        const_type: Some(GoType {
            name: "string".to_string(),
            is_pointer: false,
            is_slice: false,
        }),
        start_line: 9,
        end_line: 9,
        file_path: PathBuf::from("main.go"),
    };

    let change_target = ChangeTarget::Constant(default_name_const);
    let context = extractor
        .extract_context_for_target(change_target, &[source_file])
        .expect("Failed to extract constant context");

    // 验证基本功能
    assert_eq!(context.change_target.name(), "DefaultName");
    assert_eq!(
        context.change_target.change_type(),
        semantic_diff_core::extractor::ChangeType::Constant
    );

    // 验证统计信息
    let stats = context.get_stats();
    println!("Constant context stats: {stats:?}");
}

#[test]
fn test_context_api_methods() {
    let source_file = create_simple_test_source_file();
    let extractor = SemanticContextExtractor::new();

    let create_user_func = GoFunctionInfo {
        name: "CreateUser".to_string(),
        receiver: None,
        parameters: vec![],
        return_types: vec![],
        body: "return nil".to_string(), // 使用更简单的函数体，不会产生依赖
        start_line: 1,
        end_line: 3,
        file_path: PathBuf::from("main.go"),
    };

    let mut context = extractor
        .extract_context(&create_user_func, &[source_file])
        .expect("Failed to extract context");

    // 记录初始状态
    let initial_functions_count = context.dependent_functions.len();
    let initial_imports_count = context.imports.len();

    // 测试添加方法
    let user_type = GoTypeDefinition {
        name: "TestType".to_string(),
        kind: GoTypeKind::Struct,
        definition: "type TestType struct {}".to_string(),
        file_path: PathBuf::from("test.go"),
        dependencies: vec![],
    };
    context.add_type(user_type);

    let helper_func = GoFunctionInfo {
        name: "helper".to_string(),
        receiver: None,
        parameters: vec![],
        return_types: vec![],
        body: "return".to_string(),
        start_line: 1,
        end_line: 1,
        file_path: PathBuf::from("helper.go"),
    };
    context.add_function(helper_func);

    let test_const = GoConstantDefinition {
        name: "TestConst".to_string(),
        value: "42".to_string(),
        const_type: None,
        start_line: 1,
        end_line: 1,
        file_path: PathBuf::from("const.go"),
    };
    context.add_constant(test_const);

    let test_var = GoVariableDefinition {
        name: "TestVar".to_string(),
        var_type: None,
        initial_value: None,
        start_line: 1,
        end_line: 1,
        file_path: PathBuf::from("var.go"),
    };
    context.add_variable(test_var);

    let import = Import {
        path: "testing".to_string(),
        alias: None,
    };
    context.add_import(import);

    context.add_cross_module_dependency(
        "test".to_string(),
        vec!["dep1".to_string(), "dep2".to_string()],
    );

    // 验证添加的内容

    assert_eq!(context.related_types.len(), 1);
    assert_eq!(
        context.dependent_functions.len(),
        initial_functions_count + 1
    );
    assert_eq!(context.constants.len(), 1);
    assert_eq!(context.variables.len(), 1);
    assert_eq!(context.imports.len(), initial_imports_count + 1);
    assert_eq!(context.cross_module_dependencies.len(), 1);

    // 测试统计信息
    let stats = context.get_stats();
    assert_eq!(stats.types_count, 1);
    // functions_count 包含主函数 + 添加的函数
    assert_eq!(stats.functions_count, 1 + initial_functions_count + 1); // 1 (主函数) + 0 (初始) + 1 (添加的) = 2
    assert_eq!(stats.constants_count, 1);
    assert_eq!(stats.variables_count, 1);
    assert_eq!(stats.imports_count, initial_imports_count + 1);
    assert_eq!(stats.modules_count, 1);

    // 测试文件分组
    let types_by_file = context.get_types_by_file();
    assert!(!types_by_file.is_empty());

    let functions_by_file = context.get_functions_by_file();
    assert!(!functions_by_file.is_empty());

    // 测试涉及的文件
    let involved_files = context.get_involved_files();
    assert!(involved_files.len() >= 4); // main.go, test.go, helper.go, const.go, var.go
}

#[test]
fn test_extractor_configuration() {
    // 测试提取器的配置选项
    let extractor1 = SemanticContextExtractor::new();
    assert_eq!(extractor1.get_max_recursion_depth(), 10);

    let extractor2 =
        SemanticContextExtractor::new_with_project_path("github.com/test/project".to_string());
    assert_eq!(extractor2.get_max_recursion_depth(), 10);

    let extractor3 = SemanticContextExtractor::new().with_max_recursion_depth(5);
    assert_eq!(extractor3.get_max_recursion_depth(), 5);

    // 测试从项目根目录创建（使用临时目录）
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp directory");
    let go_mod_content = "module github.com/test/example\n\ngo 1.21\n";
    std::fs::write(temp_dir.path().join("go.mod"), go_mod_content).expect("Failed to write go.mod");

    let extractor4 = SemanticContextExtractor::from_project_root(temp_dir.path())
        .expect("Failed to create extractor from project root");
    assert_eq!(extractor4.get_max_recursion_depth(), 10);
}

#[test]
fn test_change_target_api() {
    // 测试 ChangeTarget 的API
    let func = GoFunctionInfo {
        name: "test".to_string(),
        receiver: None,
        parameters: vec![],
        return_types: vec![],
        body: "".to_string(),
        start_line: 1,
        end_line: 1,
        file_path: PathBuf::from("test.go"),
    };

    let func_target = ChangeTarget::Function(func);
    assert_eq!(func_target.name(), "test");
    assert_eq!(
        func_target.change_type(),
        semantic_diff_core::extractor::ChangeType::Function
    );
    assert_eq!(func_target.file_path(), &PathBuf::from("test.go"));

    let type_def = GoTypeDefinition {
        name: "TestType".to_string(),
        kind: GoTypeKind::Struct,
        definition: "".to_string(),
        file_path: PathBuf::from("type.go"),
        dependencies: vec![],
    };

    let type_target = ChangeTarget::Type(type_def);
    assert_eq!(type_target.name(), "TestType");
    assert_eq!(
        type_target.change_type(),
        semantic_diff_core::extractor::ChangeType::Type
    );
    assert_eq!(type_target.file_path(), &PathBuf::from("type.go"));
}
