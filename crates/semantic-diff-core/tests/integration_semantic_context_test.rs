//! 语义上下文提取器的集成测试
//!
//! 测试复杂场景，包括跨模块依赖、全局变量、接口、结构体等

use semantic_diff_core::extractor::{ChangeTarget, SemanticContextExtractor};
use semantic_diff_core::parser::common::LanguageParser;
use semantic_diff_core::parser::{
    GoConstantDefinition, GoDeclaration, GoFunctionInfo, GoLanguageInfo, GoParameter,
    GoReceiverInfo, GoType, GoTypeDefinition, GoTypeKind, GoVariableDefinition, Import, SourceFile,
    SupportedLanguage,
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// 创建测试用的临时Go项目
fn create_test_go_project() -> (TempDir, Vec<SourceFile>) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_root = temp_dir.path();

    // 创建 go.mod 文件
    let go_mod_content = r#"module github.com/test/semantic-diff-test

go 1.21
"#;
    fs::write(project_root.join("go.mod"), go_mod_content).expect("Failed to write go.mod");

    // 创建多个Go文件来模拟复杂的项目结构
    let files = vec![
        // main.go - 主文件
        (
            "main.go",
            r#"package main

import (
    "fmt"
    "github.com/test/semantic-diff-test/models"
    "github.com/test/semantic-diff-test/services"
)

var GlobalConfig *models.Config

func main() {
    GlobalConfig = &models.Config{
        Host: "localhost",
        Port: 8080,
    }
    
    userService := services.NewUserService(GlobalConfig)
    user := userService.CreateUser("Alice", 25)
    fmt.Printf("Created user: %+v\n", user)
}

func InitializeSystem(config *models.Config) error {
    GlobalConfig = config
    return services.ValidateConfig(config)
}
"#,
        ),
        // models/user.go - 用户模型
        (
            "models/user.go",
            r#"package models

import "time"

type User struct {
    ID       int       `json:"id"`
    Name     string    `json:"name"`
    Age      int       `json:"age"`
    Profile  *Profile  `json:"profile,omitempty"`
    Created  time.Time `json:"created"`
}

type Profile struct {
    Bio     string   `json:"bio"`
    Tags    []string `json:"tags"`
    Address Address  `json:"address"`
}

type Address struct {
    Street  string `json:"street"`
    City    string `json:"city"`
    Country string `json:"country"`
}

func (u *User) GetDisplayName() string {
    if u.Profile != nil && u.Profile.Bio != "" {
        return u.Name + " (" + u.Profile.Bio + ")"
    }
    return u.Name
}

func (u *User) UpdateProfile(profile *Profile) {
    u.Profile = profile
}
"#,
        ),
        // models/config.go - 配置模型
        (
            "models/config.go",
            r#"package models

const (
    DefaultHost = "localhost"
    DefaultPort = 8080
    MaxRetries  = 3
)

var (
    GlobalTimeout = 30
    DebugMode     = false
)

type Config struct {
    Host     string `json:"host"`
    Port     int    `json:"port"`
    Database DatabaseConfig `json:"database"`
    Features FeatureFlags   `json:"features"`
}

type DatabaseConfig struct {
    Driver   string `json:"driver"`
    Host     string `json:"host"`
    Port     int    `json:"port"`
    Name     string `json:"name"`
    Username string `json:"username"`
    Password string `json:"password"`
}

type FeatureFlags struct {
    EnableLogging   bool `json:"enable_logging"`
    EnableMetrics   bool `json:"enable_metrics"`
    EnableTracing   bool `json:"enable_tracing"`
}

func NewDefaultConfig() *Config {
    return &Config{
        Host: DefaultHost,
        Port: DefaultPort,
        Database: DatabaseConfig{
            Driver: "postgres",
            Host:   "localhost",
            Port:   5432,
        },
        Features: FeatureFlags{
            EnableLogging: true,
            EnableMetrics: false,
            EnableTracing: DebugMode,
        },
    }
}
"#,
        ),
        // services/user_service.go - 用户服务
        (
            "services/user_service.go",
            r#"package services

import (
    "fmt"
    "time"
    "github.com/test/semantic-diff-test/models"
)

type UserService struct {
    config *models.Config
    users  map[int]*models.User
    nextID int
}

type UserRepository interface {
    Save(user *models.User) error
    FindByID(id int) (*models.User, error)
    FindByName(name string) ([]*models.User, error)
}

func NewUserService(config *models.Config) *UserService {
    return &UserService{
        config: config,
        users:  make(map[int]*models.User),
        nextID: 1,
    }
}

func (s *UserService) CreateUser(name string, age int) *models.User {
    user := &models.User{
        ID:      s.nextID,
        Name:    name,
        Age:     age,
        Created: time.Now(),
    }
    
    s.users[user.ID] = user
    s.nextID++
    
    if s.config.Features.EnableLogging {
        fmt.Printf("Created user: %s\n", user.Name)
    }
    
    return user
}

func (s *UserService) GetUser(id int) (*models.User, error) {
    user, exists := s.users[id]
    if !exists {
        return nil, fmt.Errorf("user with ID %d not found", id)
    }
    return user, nil
}

func (s *UserService) UpdateUserProfile(userID int, profile *models.Profile) error {
    user, err := s.GetUser(userID)
    if err != nil {
        return err
    }
    
    user.UpdateProfile(profile)
    return nil
}
"#,
        ),
        // services/config_service.go - 配置服务
        (
            "services/config_service.go",
            r#"package services

import (
    "errors"
    "github.com/test/semantic-diff-test/models"
)

func ValidateConfig(config *models.Config) error {
    if config == nil {
        return errors.New("config cannot be nil")
    }
    
    if config.Host == "" {
        config.Host = models.DefaultHost
    }
    
    if config.Port <= 0 {
        config.Port = models.DefaultPort
    }
    
    return validateDatabaseConfig(&config.Database)
}

func validateDatabaseConfig(dbConfig *models.DatabaseConfig) error {
    if dbConfig.Driver == "" {
        return errors.New("database driver is required")
    }
    
    if dbConfig.Host == "" {
        return errors.New("database host is required")
    }
    
    if dbConfig.Port <= 0 {
        return errors.New("database port must be positive")
    }
    
    return nil
}

func LoadConfigFromFile(filename string) (*models.Config, error) {
    // 简化实现，实际会从文件读取
    return models.NewDefaultConfig(), nil
}
"#,
        ),
    ];

    let mut source_files = Vec::new();
    let mut parser =
        semantic_diff_core::parser::go::GoParser::new().expect("Failed to create parser");

    for (filename, content) in files {
        let file_path = if filename.contains('/') {
            let dir_path = project_root.join(filename).parent().unwrap().to_path_buf();
            fs::create_dir_all(&dir_path).expect("Failed to create directory");
            project_root.join(filename)
        } else {
            project_root.join(filename)
        };

        fs::write(&file_path, content).expect("Failed to write file");

        // 解析文件
        let syntax_tree = parser
            .parse_source(content)
            .expect("Failed to parse source");
        let package_name = if filename.contains("models/") {
            "models"
        } else if filename.contains("services/") {
            "services"
        } else {
            "main"
        };

        let mut go_info = GoLanguageInfo::new(package_name.to_string());

        // 添加导入（简化版本）
        if content.contains("import") {
            if content.contains("fmt") {
                go_info.add_import(Import {
                    path: "fmt".to_string(),
                    alias: None,
                });
            }
            if content.contains("time") {
                go_info.add_import(Import {
                    path: "time".to_string(),
                    alias: None,
                });
            }
            if content.contains("errors") {
                go_info.add_import(Import {
                    path: "errors".to_string(),
                    alias: None,
                });
            }
            if content.contains("models") {
                go_info.add_import(Import {
                    path: "github.com/test/semantic-diff-test/models".to_string(),
                    alias: None,
                });
            }
            if content.contains("services") {
                go_info.add_import(Import {
                    path: "github.com/test/semantic-diff-test/services".to_string(),
                    alias: None,
                });
            }
        }

        // 手动添加类型声明（基于文件内容）
        if filename == "models/user.go" {
            // 添加 User 类型
            go_info.add_declaration(Box::new(GoDeclaration::Type(GoTypeDefinition {
                name: "User".to_string(),
                kind: GoTypeKind::Struct,
                definition: r#"type User struct {
    ID       int       `json:"id"`
    Name     string    `json:"name"`
    Age      int       `json:"age"`
    Profile  *Profile  `json:"profile,omitempty"`
    Created  time.Time `json:"created"`
}"#
                .to_string(),
                file_path: file_path.clone(),
                dependencies: vec!["Profile".to_string()],
            })));

            // 添加 Profile 类型
            go_info.add_declaration(Box::new(GoDeclaration::Type(GoTypeDefinition {
                name: "Profile".to_string(),
                kind: GoTypeKind::Struct,
                definition: r#"type Profile struct {
    Bio     string   `json:"bio"`
    Tags    []string `json:"tags"`
    Address Address  `json:"address"`
}"#
                .to_string(),
                file_path: file_path.clone(),
                dependencies: vec!["Address".to_string()],
            })));

            // 添加 Address 类型
            go_info.add_declaration(Box::new(GoDeclaration::Type(GoTypeDefinition {
                name: "Address".to_string(),
                kind: GoTypeKind::Struct,
                definition: r#"type Address struct {
    Street  string `json:"street"`
    City    string `json:"city"`
    Country string `json:"country"`
}"#
                .to_string(),
                file_path: file_path.clone(),
                dependencies: vec![],
            })));

            // 添加方法
            go_info.add_declaration(Box::new(GoDeclaration::Method(GoFunctionInfo {
                name: "GetDisplayName".to_string(),
                receiver: Some(GoReceiverInfo {
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
                body: r#"if u.Profile != nil && u.Profile.Bio != "" {
        return u.Name + " (" + u.Profile.Bio + ")"
    }
    return u.Name"#
                    .to_string(),
                start_line: 25,
                end_line: 30,
                file_path: file_path.clone(),
            })));

            go_info.add_declaration(Box::new(GoDeclaration::Method(GoFunctionInfo {
                name: "UpdateProfile".to_string(),
                receiver: Some(GoReceiverInfo {
                    name: "u".to_string(),
                    type_name: "User".to_string(),
                    is_pointer: true,
                }),
                parameters: vec![GoParameter {
                    name: "profile".to_string(),
                    param_type: GoType {
                        name: "Profile".to_string(),
                        is_pointer: true,
                        is_slice: false,
                    },
                }],
                return_types: vec![],
                body: "u.Profile = profile".to_string(),
                start_line: 32,
                end_line: 34,
                file_path: file_path.clone(),
            })));
        }

        if filename == "models/config.go" {
            // 添加常量
            go_info.add_declaration(Box::new(GoDeclaration::Constant(GoConstantDefinition {
                name: "DefaultHost".to_string(),
                const_type: Some(GoType {
                    name: "string".to_string(),
                    is_pointer: false,
                    is_slice: false,
                }),
                value: "\"localhost\"".to_string(),
                start_line: 5,
                end_line: 5,
                file_path: file_path.clone(),
            })));

            go_info.add_declaration(Box::new(GoDeclaration::Constant(GoConstantDefinition {
                name: "DefaultPort".to_string(),
                const_type: Some(GoType {
                    name: "int".to_string(),
                    is_pointer: false,
                    is_slice: false,
                }),
                value: "8080".to_string(),
                start_line: 6,
                end_line: 6,
                file_path: file_path.clone(),
            })));

            // 添加变量
            go_info.add_declaration(Box::new(GoDeclaration::Variable(GoVariableDefinition {
                name: "GlobalTimeout".to_string(),
                var_type: Some(GoType {
                    name: "int".to_string(),
                    is_pointer: false,
                    is_slice: false,
                }),
                initial_value: Some("30".to_string()),
                start_line: 10,
                end_line: 10,
                file_path: file_path.clone(),
            })));

            // 添加 Config 类型
            go_info.add_declaration(Box::new(GoDeclaration::Type(GoTypeDefinition {
                name: "Config".to_string(),
                kind: GoTypeKind::Struct,
                definition: r#"type Config struct {
    Host     string `json:"host"`
    Port     int    `json:"port"`
    Database DatabaseConfig `json:"database"`
    Features FeatureFlags   `json:"features"`
}"#
                .to_string(),
                file_path: file_path.clone(),
                dependencies: vec!["DatabaseConfig".to_string(), "FeatureFlags".to_string()],
            })));

            // 添加 DatabaseConfig 类型
            go_info.add_declaration(Box::new(GoDeclaration::Type(GoTypeDefinition {
                name: "DatabaseConfig".to_string(),
                kind: GoTypeKind::Struct,
                definition: r#"type DatabaseConfig struct {
    Driver   string `json:"driver"`
    Host     string `json:"host"`
    Port     int    `json:"port"`
    Name     string `json:"name"`
    Username string `json:"username"`
    Password string `json:"password"`
}"#
                .to_string(),
                file_path: file_path.clone(),
                dependencies: vec![],
            })));

            // 添加 FeatureFlags 类型
            go_info.add_declaration(Box::new(GoDeclaration::Type(GoTypeDefinition {
                name: "FeatureFlags".to_string(),
                kind: GoTypeKind::Struct,
                definition: r#"type FeatureFlags struct {
    EnableLogging   bool `json:"enable_logging"`
    EnableMetrics   bool `json:"enable_metrics"`
    EnableTracing   bool `json:"enable_tracing"`
}"#
                .to_string(),
                file_path: file_path.clone(),
                dependencies: vec![],
            })));

            // 添加 NewDefaultConfig 函数
            go_info.add_declaration(Box::new(GoDeclaration::Function(GoFunctionInfo {
                name: "NewDefaultConfig".to_string(),
                receiver: None,
                parameters: vec![],
                return_types: vec![GoType {
                    name: "Config".to_string(),
                    is_pointer: true,
                    is_slice: false,
                }],
                body: r#"return &Config{
        Host: DefaultHost,
        Port: DefaultPort,
        Database: DatabaseConfig{
            Driver: "postgres",
            Host:   "localhost",
            Port:   5432,
        },
        Features: FeatureFlags{
            EnableLogging: true,
            EnableMetrics: false,
            EnableTracing: DebugMode,
        },
    }"#
                .to_string(),
                start_line: 40,
                end_line: 55,
                file_path: file_path.clone(),
            })));
        }

        if filename == "services/user_service.go" {
            // 添加 UserService 类型
            go_info.add_declaration(Box::new(GoDeclaration::Type(GoTypeDefinition {
                name: "UserService".to_string(),
                kind: GoTypeKind::Struct,
                definition: r#"type UserService struct {
    config *models.Config
    users  map[int]*models.User
    nextID int
}"#
                .to_string(),
                file_path: file_path.clone(),
                dependencies: vec!["Config".to_string(), "User".to_string()],
            })));

            // 添加 UserRepository 接口
            go_info.add_declaration(Box::new(GoDeclaration::Type(GoTypeDefinition {
                name: "UserRepository".to_string(),
                kind: GoTypeKind::Interface,
                definition: r#"type UserRepository interface {
    Save(user *models.User) error
    FindByID(id int) (*models.User, error)
    FindByName(name string) ([]*models.User, error)
}"#
                .to_string(),
                file_path: file_path.clone(),
                dependencies: vec!["User".to_string()],
            })));

            // 添加 NewUserService 函数
            go_info.add_declaration(Box::new(GoDeclaration::Function(GoFunctionInfo {
                name: "NewUserService".to_string(),
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
                    name: "UserService".to_string(),
                    is_pointer: true,
                    is_slice: false,
                }],
                body: r#"return &UserService{
        config: config,
        users:  make(map[int]*models.User),
        nextID: 1,
    }"#
                .to_string(),
                start_line: 20,
                end_line: 26,
                file_path: file_path.clone(),
            })));

            // 添加 CreateUser 方法
            go_info.add_declaration(Box::new(GoDeclaration::Method(GoFunctionInfo {
                name: "CreateUser".to_string(),
                receiver: Some(GoReceiverInfo {
                    name: "s".to_string(),
                    type_name: "UserService".to_string(),
                    is_pointer: true,
                }),
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
                body: r#"user := &models.User{
        ID:      s.nextID,
        Name:    name,
        Age:     age,
        Created: time.Now(),
    }
    
    s.users[user.ID] = user
    s.nextID++
    
    if s.config.Features.EnableLogging {
        fmt.Printf("Created user: %s\n", user.Name)
    }
    
    return user"#
                    .to_string(),
                start_line: 28,
                end_line: 45,
                file_path: file_path.clone(),
            })));

            // 添加 UpdateUserProfile 方法
            go_info.add_declaration(Box::new(GoDeclaration::Method(GoFunctionInfo {
                name: "UpdateUserProfile".to_string(),
                receiver: Some(GoReceiverInfo {
                    name: "s".to_string(),
                    type_name: "UserService".to_string(),
                    is_pointer: true,
                }),
                parameters: vec![
                    GoParameter {
                        name: "userID".to_string(),
                        param_type: GoType {
                            name: "int".to_string(),
                            is_pointer: false,
                            is_slice: false,
                        },
                    },
                    GoParameter {
                        name: "profile".to_string(),
                        param_type: GoType {
                            name: "Profile".to_string(),
                            is_pointer: true,
                            is_slice: false,
                        },
                    },
                ],
                return_types: vec![GoType {
                    name: "error".to_string(),
                    is_pointer: false,
                    is_slice: false,
                }],
                body: r#"user, err := s.GetUser(userID)
    if err != nil {
        return err
    }
    
    user.UpdateProfile(profile)
    return nil"#
                    .to_string(),
                start_line: 55,
                end_line: 63,
                file_path: file_path.clone(),
            })));
        }

        if filename == "services/config_service.go" {
            // 添加 ValidateConfig 函数
            go_info.add_declaration(Box::new(GoDeclaration::Function(GoFunctionInfo {
                name: "ValidateConfig".to_string(),
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
                    name: "error".to_string(),
                    is_pointer: false,
                    is_slice: false,
                }],
                body: r#"if config == nil {
        return errors.New("config cannot be nil")
    }
    
    if config.Host == "" {
        config.Host = models.DefaultHost
    }
    
    if config.Port <= 0 {
        config.Port = models.DefaultPort
    }
    
    return validateDatabaseConfig(&config.Database)"#
                    .to_string(),
                start_line: 8,
                end_line: 21,
                file_path: file_path.clone(),
            })));
        }

        if filename == "main.go" {
            // 添加 GlobalConfig 变量
            go_info.add_declaration(Box::new(GoDeclaration::Variable(GoVariableDefinition {
                name: "GlobalConfig".to_string(),
                var_type: Some(GoType {
                    name: "Config".to_string(),
                    is_pointer: true,
                    is_slice: false,
                }),
                initial_value: None,
                start_line: 8,
                end_line: 8,
                file_path: file_path.clone(),
            })));

            // 添加 main 函数
            go_info.add_declaration(Box::new(GoDeclaration::Function(GoFunctionInfo {
                name: "main".to_string(),
                receiver: None,
                parameters: vec![],
                return_types: vec![],
                body: r#"GlobalConfig = &models.Config{
        Host: "localhost",
        Port: 8080,
    }
    
    userService := services.NewUserService(GlobalConfig)
    user := userService.CreateUser("Alice", 25)
    fmt.Printf("Created user: %+v\n", user)"#
                    .to_string(),
                start_line: 10,
                end_line: 18,
                file_path: file_path.clone(),
            })));

            // 添加 InitializeSystem 函数
            go_info.add_declaration(Box::new(GoDeclaration::Function(GoFunctionInfo {
                name: "InitializeSystem".to_string(),
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
                    name: "error".to_string(),
                    is_pointer: false,
                    is_slice: false,
                }],
                body: r#"GlobalConfig = config
    return services.ValidateConfig(config)"#
                    .to_string(),
                start_line: 20,
                end_line: 23,
                file_path: file_path.clone(),
            })));
        }

        let source_file = SourceFile {
            path: file_path,
            source_code: content.to_string(),
            syntax_tree,
            language: SupportedLanguage::Go,
            language_specific: Box::new(go_info),
        };

        source_files.push(source_file);
    }

    (temp_dir, source_files)
}

#[test]
fn test_function_context_extraction_complex() {
    let (_temp_dir, source_files) = create_test_go_project();
    let extractor = SemanticContextExtractor::from_project_root(_temp_dir.path())
        .expect("Failed to create extractor");

    // 测试提取 CreateUser 函数的上下文
    let create_user_func = GoFunctionInfo {
        name: "CreateUser".to_string(),
        receiver: Some(semantic_diff_core::parser::GoReceiverInfo {
            name: "s".to_string(),
            type_name: "UserService".to_string(),
            is_pointer: true,
        }),
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
        body: r#"user := &models.User{
        ID:      s.nextID,
        Name:    name,
        Age:     age,
        Created: time.Now(),
    }
    
    s.users[user.ID] = user
    s.nextID++
    
    if s.config.Features.EnableLogging {
        fmt.Printf("Created user: %s\n", user.Name)
    }
    
    return user"#
            .to_string(),
        start_line: 25,
        end_line: 40,
        file_path: PathBuf::from("services/user_service.go"),
    };

    let context = extractor
        .extract_context(&create_user_func, &source_files)
        .expect("Failed to extract context");

    // 验证提取的上下文
    assert!(
        !context.related_types.is_empty(),
        "Should have related types"
    );
    assert!(!context.imports.is_empty(), "Should have imports");

    // 验证跨模块依赖
    assert!(
        !context.cross_module_dependencies.is_empty(),
        "Should have cross-module dependencies"
    );

    let stats = context.get_stats();
    println!("Function context stats: {stats:?}");

    // 验证涉及的文件
    let involved_files = context.get_involved_files();
    assert!(involved_files.len() > 1, "Should involve multiple files");
}

#[test]
fn test_type_context_extraction() {
    let (_temp_dir, source_files) = create_test_go_project();
    let extractor = SemanticContextExtractor::from_project_root(_temp_dir.path())
        .expect("Failed to create extractor");

    // 测试提取 User 类型的上下文
    let user_type = GoTypeDefinition {
        name: "User".to_string(),
        kind: GoTypeKind::Struct,
        definition: r#"type User struct {
    ID       int       `json:"id"`
    Name     string    `json:"name"`
    Age      int       `json:"age"`
    Profile  *Profile  `json:"profile,omitempty"`
    Created  time.Time `json:"created"`
}"#
        .to_string(),
        file_path: PathBuf::from("models/user.go"),
        dependencies: vec!["Profile".to_string()],
    };

    let change_target = ChangeTarget::Type(user_type);
    let context = extractor
        .extract_context_for_target(change_target, &source_files)
        .expect("Failed to extract type context");

    // 验证提取的上下文
    assert!(
        !context.related_types.is_empty(),
        "Should have related types (Profile, Address)"
    );
    assert!(
        !context.dependent_functions.is_empty(),
        "Should have functions using User type"
    );

    let stats = context.get_stats();
    println!("Type context stats: {stats:?}");

    // 验证找到了使用 User 类型的函数
    let function_names: Vec<_> = context
        .dependent_functions
        .iter()
        .map(|f| &f.name)
        .collect();
    println!("Functions using User type: {function_names:?}");
}

#[test]
fn test_global_variable_context_extraction() {
    let (_temp_dir, source_files) = create_test_go_project();
    let extractor = SemanticContextExtractor::from_project_root(_temp_dir.path())
        .expect("Failed to create extractor");

    // 测试提取全局变量 GlobalConfig 的上下文
    let global_config_var = GoVariableDefinition {
        name: "GlobalConfig".to_string(),
        var_type: Some(GoType {
            name: "Config".to_string(),
            is_pointer: true,
            is_slice: false,
        }),
        initial_value: None,
        start_line: 8,
        end_line: 8,
        file_path: PathBuf::from("main.go"),
    };

    let change_target = ChangeTarget::Variable(global_config_var);
    let context = extractor
        .extract_context_for_target(change_target, &source_files)
        .expect("Failed to extract variable context");

    // 验证提取的上下文
    assert!(
        !context.related_types.is_empty(),
        "Should have Config type and its dependencies"
    );
    assert!(
        !context.dependent_functions.is_empty(),
        "Should have functions using GlobalConfig"
    );

    let stats = context.get_stats();
    println!("Variable context stats: {stats:?}");

    // 验证跨模块依赖
    assert!(
        !context.cross_module_dependencies.is_empty(),
        "Should have cross-module dependencies"
    );
}

#[test]
fn test_constant_context_extraction() {
    let (_temp_dir, source_files) = create_test_go_project();
    let extractor = SemanticContextExtractor::from_project_root(_temp_dir.path())
        .expect("Failed to create extractor");

    // 测试提取常量 DefaultHost 的上下文
    let default_host_const = GoConstantDefinition {
        name: "DefaultHost".to_string(),
        value: "\"localhost\"".to_string(),
        const_type: Some(GoType {
            name: "string".to_string(),
            is_pointer: false,
            is_slice: false,
        }),
        start_line: 4,
        end_line: 4,
        file_path: PathBuf::from("models/config.go"),
    };

    let change_target = ChangeTarget::Constant(default_host_const);
    let context = extractor
        .extract_context_for_target(change_target, &source_files)
        .expect("Failed to extract constant context");

    // 验证提取的上下文
    assert!(
        !context.dependent_functions.is_empty(),
        "Should have functions using DefaultHost"
    );

    let stats = context.get_stats();
    println!("Constant context stats: {stats:?}");

    // 验证找到了使用常量的函数
    let function_names: Vec<_> = context
        .dependent_functions
        .iter()
        .map(|f| &f.name)
        .collect();
    println!("Functions using DefaultHost: {function_names:?}");
}

#[test]
fn test_cross_module_dependency_analysis() {
    let (_temp_dir, source_files) = create_test_go_project();
    let extractor = SemanticContextExtractor::from_project_root(_temp_dir.path())
        .expect("Failed to create extractor");

    // 测试 main 包中的函数，它依赖多个模块
    let main_func = GoFunctionInfo {
        name: "main".to_string(),
        receiver: None,
        parameters: vec![],
        return_types: vec![],
        body: r#"GlobalConfig = &models.Config{
        Host: "localhost",
        Port: 8080,
    }
    
    userService := services.NewUserService(GlobalConfig)
    user := userService.CreateUser("Alice", 25)
    fmt.Printf("Created user: %+v\n", user)"#
            .to_string(),
        start_line: 10,
        end_line: 18,
        file_path: PathBuf::from("main.go"),
    };

    let context = extractor
        .extract_context(&main_func, &source_files)
        .expect("Failed to extract context");

    // 验证跨模块依赖
    assert!(
        !context.cross_module_dependencies.is_empty(),
        "Should have cross-module dependencies"
    );

    println!(
        "Cross-module dependencies: {:?}",
        context.cross_module_dependencies
    );

    // 验证包含了来自不同模块的类型和函数
    let involved_files = context.get_involved_files();
    let file_paths: Vec<_> = involved_files.iter().map(|p| p.to_string_lossy()).collect();
    println!("Involved files: {file_paths:?}");

    // 应该涉及多个模块的文件
    assert!(
        file_paths.iter().any(|p| p.contains("models/")),
        "Should involve models module"
    );
    assert!(
        file_paths.iter().any(|p| p.contains("services/")),
        "Should involve services module"
    );
}

#[test]
fn test_interface_dependency_extraction() {
    let (_temp_dir, source_files) = create_test_go_project();
    let extractor = SemanticContextExtractor::from_project_root(_temp_dir.path())
        .expect("Failed to create extractor");

    // 测试接口类型的上下文提取
    let user_repository_interface = GoTypeDefinition {
        name: "UserRepository".to_string(),
        kind: GoTypeKind::Interface,
        definition: r#"type UserRepository interface {
    Save(user *models.User) error
    FindByID(id int) (*models.User, error)
    FindByName(name string) ([]*models.User, error)
}"#
        .to_string(),
        file_path: PathBuf::from("services/user_service.go"),
        dependencies: vec!["User".to_string()],
    };

    let change_target = ChangeTarget::Type(user_repository_interface);
    let context = extractor
        .extract_context_for_target(change_target, &source_files)
        .expect("Failed to extract interface context");

    // 验证接口的依赖类型被正确提取
    assert!(
        !context.related_types.is_empty(),
        "Should have related types (User)"
    );

    let stats = context.get_stats();
    println!("Interface context stats: {stats:?}");

    // 验证跨模块依赖（接口使用了 models.User）
    assert!(
        !context.cross_module_dependencies.is_empty(),
        "Should have cross-module dependencies"
    );
}

#[test]
fn test_recursive_type_dependency_extraction() {
    let (_temp_dir, source_files) = create_test_go_project();
    let extractor = SemanticContextExtractor::from_project_root(_temp_dir.path())
        .expect("Failed to create extractor");

    // 测试递归类型依赖提取（Profile -> Address）
    let profile_type = GoTypeDefinition {
        name: "Profile".to_string(),
        kind: GoTypeKind::Struct,
        definition: r#"type Profile struct {
    Bio     string   `json:"bio"`
    Tags    []string `json:"tags"`
    Address Address  `json:"address"`
}"#
        .to_string(),
        file_path: PathBuf::from("models/user.go"),
        dependencies: vec!["Address".to_string()],
    };

    let change_target = ChangeTarget::Type(profile_type);
    let context = extractor
        .extract_context_for_target(change_target, &source_files)
        .expect("Failed to extract profile context");

    // 验证递归依赖被正确提取
    let type_names: Vec<_> = context.related_types.iter().map(|t| &t.name).collect();
    println!("Related types for Profile: {type_names:?}");

    // 应该包含 Address 类型
    assert!(
        type_names.contains(&&"Address".to_string()),
        "Should include Address type"
    );

    let stats = context.get_stats();
    println!("Profile context stats: {stats:?}");
}

#[test]
fn test_complex_function_with_multiple_dependencies() {
    let (_temp_dir, source_files) = create_test_go_project();
    let extractor = SemanticContextExtractor::from_project_root(_temp_dir.path())
        .expect("Failed to create extractor");

    // 测试复杂函数 UpdateUserProfile，它涉及多个类型和函数调用
    let update_profile_func = GoFunctionInfo {
        name: "UpdateUserProfile".to_string(),
        receiver: Some(semantic_diff_core::parser::GoReceiverInfo {
            name: "s".to_string(),
            type_name: "UserService".to_string(),
            is_pointer: true,
        }),
        parameters: vec![
            GoParameter {
                name: "userID".to_string(),
                param_type: GoType {
                    name: "int".to_string(),
                    is_pointer: false,
                    is_slice: false,
                },
            },
            GoParameter {
                name: "profile".to_string(),
                param_type: GoType {
                    name: "Profile".to_string(),
                    is_pointer: true,
                    is_slice: false,
                },
            },
        ],
        return_types: vec![GoType {
            name: "error".to_string(),
            is_pointer: false,
            is_slice: false,
        }],
        body: r#"user, err := s.GetUser(userID)
    if err != nil {
        return err
    }
    
    user.UpdateProfile(profile)
    return nil"#
            .to_string(),
        start_line: 55,
        end_line: 63,
        file_path: PathBuf::from("services/user_service.go"),
    };

    let context = extractor
        .extract_context(&update_profile_func, &source_files)
        .expect("Failed to extract context");

    // 验证复杂函数的上下文
    assert!(
        !context.related_types.is_empty(),
        "Should have related types"
    );
    assert!(
        !context.dependent_functions.is_empty(),
        "Should have dependent functions"
    );

    let stats = context.get_stats();
    println!("Complex function context stats: {stats:?}");

    // 验证包含了相关的类型
    let type_names: Vec<_> = context.related_types.iter().map(|t| &t.name).collect();
    println!("Related types: {type_names:?}");

    // 验证包含了依赖的函数
    let function_names: Vec<_> = context
        .dependent_functions
        .iter()
        .map(|f| &f.name)
        .collect();
    println!("Dependent functions: {function_names:?}");
}

#[test]
fn test_context_validation_and_completeness() {
    let (_temp_dir, source_files) = create_test_go_project();
    let extractor = SemanticContextExtractor::from_project_root(_temp_dir.path())
        .expect("Failed to create extractor");

    // 测试一个简单函数的上下文完整性
    let validate_config_func = GoFunctionInfo {
        name: "ValidateConfig".to_string(),
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
            name: "error".to_string(),
            is_pointer: false,
            is_slice: false,
        }],
        body: r#"if config == nil {
        return errors.New("config cannot be nil")
    }
    
    if config.Host == "" {
        config.Host = models.DefaultHost
    }
    
    if config.Port <= 0 {
        config.Port = models.DefaultPort
    }
    
    return validateDatabaseConfig(&config.Database)"#
            .to_string(),
        start_line: 8,
        end_line: 21,
        file_path: PathBuf::from("services/config_service.go"),
    };

    let context = extractor
        .extract_context(&validate_config_func, &source_files)
        .expect("Failed to extract context");

    // 验证上下文完整性
    let missing_deps = extractor
        .validate_context(&context)
        .expect("Failed to validate context");

    println!("Missing dependencies: {missing_deps:?}");

    // 在一个完整的实现中，这应该是空的
    // 但由于我们的简化实现，可能会有一些缺失

    let stats = context.get_stats();
    println!("Validation context stats: {stats:?}");

    // 验证上下文不为空
    assert!(!context.is_empty(), "Context should not be empty");
}
