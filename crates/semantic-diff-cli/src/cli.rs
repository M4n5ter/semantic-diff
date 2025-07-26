//! 命令行接口模块
//!
//! 提供命令行参数解析和用户交互功能

use clap::{Parser, ValueEnum};
use semantic_diff_core::{HighlightStyle, OutputFormat, Result, SemanticDiffError};
use std::path::PathBuf;

/// semantic-diff - 语义代码差异分析工具
///
/// 这是一个基于 Tree-sitter 的多语言代码差异分析工具，
/// 能够提供比传统 git diff 更丰富的语义上下文信息。
#[derive(Parser, Debug)]
#[command(name = "semantic-diff")]
#[command(author = "semantic-diff contributors")]
#[command(version = "0.1.0")]
#[command(about = "A semantic code diff tool that provides rich contextual information")]
#[command(
    long_about = "semantic-diff analyzes Git commits and provides semantic context for code changes, including related type definitions, function dependencies, and complete code slices that are self-contained and compilable."
)]
pub struct Cli {
    /// Git 提交哈希 (支持完整或短格式)
    #[arg(
        help = "Git commit hash to analyze (supports both full and short format)",
        value_name = "COMMIT_HASH"
    )]
    pub commit_hash: String,

    /// 输出格式
    #[arg(
        short = 'f',
        long = "format",
        value_enum,
        default_value_t = OutputFormatArg::PlainText,
        help = "Output format for the analysis results"
    )]
    pub format: OutputFormatArg,

    /// 高亮样式
    #[arg(
        long = "highlight",
        value_enum,
        default_value_t = HighlightStyleArg::Inline,
        help = "Syntax highlighting style for code output"
    )]
    pub highlight: HighlightStyleArg,

    /// 仓库路径
    #[arg(
        short = 'r',
        long = "repo",
        default_value = ".",
        help = "Path to the Git repository",
        value_name = "PATH"
    )]
    pub repo_path: PathBuf,

    /// 是否包含注释
    #[arg(
        long = "include-comments",
        default_value_t = true,
        help = "Include comments in the generated code slices"
    )]
    pub include_comments: bool,

    /// 最大依赖深度
    #[arg(
        long = "max-depth",
        default_value_t = 3,
        value_name = "DEPTH",
        help = "Maximum depth for dependency resolution (1-10)",
        value_parser = clap::value_parser!(u32).range(1..=10)
    )]
    pub max_depth: u32,

    /// 排除测试文件
    #[arg(long = "exclude-tests", help = "Exclude test files from analysis")]
    pub exclude_tests: bool,

    /// 详细输出
    #[arg(short = 'v', long = "verbose", help = "Enable verbose logging output")]
    pub verbose: bool,

    /// 输出到文件
    #[arg(
        short = 'o',
        long = "output",
        value_name = "FILE",
        help = "Write output to a file instead of stdout"
    )]
    pub output_file: Option<PathBuf>,

    /// 只显示变更的函数
    #[arg(
        long = "functions-only",
        help = "Only show changed functions without their dependencies"
    )]
    pub functions_only: bool,

    /// 最大输出行数
    #[arg(
        long = "max-lines",
        value_name = "LINES",
        help = "Maximum number of lines in the output (0 for unlimited)",
        value_parser = clap::value_parser!(u32)
    )]
    pub max_lines: Option<u32>,

    /// 显示依赖图
    #[arg(
        long = "show-dependencies",
        help = "Include dependency graph in the output"
    )]
    pub show_dependencies: bool,
}

/// 输出格式命令行参数
#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormatArg {
    /// 纯文本格式输出
    #[value(name = "text")]
    PlainText,
    /// Markdown 格式输出
    #[value(name = "markdown")]
    Markdown,
    /// HTML 格式输出
    #[value(name = "html")]
    Html,
}

/// 高亮样式命令行参数
#[derive(Debug, Clone, ValueEnum)]
pub enum HighlightStyleArg {
    /// 无语法高亮
    #[value(name = "none")]
    None,
    /// 内联语法高亮
    #[value(name = "inline")]
    Inline,
    /// 分离的语法高亮块
    #[value(name = "separate")]
    Separate,
}

/// 应用程序配置信息
#[derive(Debug, Clone)]
pub struct Config {
    /// Git 提交哈希
    pub commit_hash: String,
    /// 输出格式
    pub output_format: OutputFormat,
    /// 是否包含注释
    pub include_comments: bool,
    /// 最大依赖深度
    pub max_dependency_depth: u32,
    /// 是否排除测试文件
    pub exclude_test_files: bool,
    /// 高亮样式
    pub highlight_style: HighlightStyle,
    /// 仓库路径
    pub repo_path: PathBuf,
    /// 是否启用详细输出
    pub verbose: bool,
    /// 输出文件路径
    pub output_file: Option<PathBuf>,
    /// 是否只显示函数
    pub functions_only: bool,
    /// 最大输出行数
    pub max_lines: Option<u32>,
    /// 是否显示依赖图
    pub show_dependencies: bool,
}

impl From<OutputFormatArg> for OutputFormat {
    fn from(arg: OutputFormatArg) -> Self {
        match arg {
            OutputFormatArg::PlainText => OutputFormat::PlainText,
            OutputFormatArg::Markdown => OutputFormat::Markdown,
            OutputFormatArg::Html => OutputFormat::Html,
        }
    }
}

impl From<HighlightStyleArg> for HighlightStyle {
    fn from(arg: HighlightStyleArg) -> Self {
        match arg {
            HighlightStyleArg::None => HighlightStyle::None,
            HighlightStyleArg::Inline => HighlightStyle::Inline,
            HighlightStyleArg::Separate => HighlightStyle::Separate,
        }
    }
}

impl From<Cli> for Config {
    fn from(cli: Cli) -> Self {
        Config {
            commit_hash: cli.commit_hash,
            output_format: cli.format.into(),
            include_comments: cli.include_comments,
            max_dependency_depth: cli.max_depth,
            exclude_test_files: cli.exclude_tests,
            highlight_style: cli.highlight.into(),
            repo_path: cli.repo_path,
            verbose: cli.verbose,
            output_file: cli.output_file,
            functions_only: cli.functions_only,
            max_lines: cli.max_lines,
            show_dependencies: cli.show_dependencies,
        }
    }
}

impl Cli {
    /// 解析命令行参数
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// 验证参数的有效性
    pub fn validate(&self) -> Result<()> {
        // 验证提交哈希
        if self.commit_hash.is_empty() {
            return Err(SemanticDiffError::InvalidCommitHash(
                "Commit hash cannot be empty".to_string(),
            ));
        }

        // 验证提交哈希格式 (Git 哈希应该是 7-40 个十六进制字符)
        if !self.commit_hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(SemanticDiffError::InvalidCommitHash(format!(
                "Invalid commit hash format: {}",
                self.commit_hash
            )));
        }

        let hash_len = self.commit_hash.len();
        if !(7..=40).contains(&hash_len) {
            return Err(SemanticDiffError::InvalidCommitHash(format!(
                "Commit hash length must be between 7 and 40 characters, got {hash_len}"
            )));
        }

        // 验证仓库路径
        if !self.repo_path.exists() {
            return Err(SemanticDiffError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "Repository path does not exist: {}",
                    self.repo_path.display()
                ),
            )));
        }

        // 验证并创建输出文件路径 (如果指定)
        if let Some(output_file) = &self.output_file {
            if let Some(parent) = output_file.parent() {
                // 只有当父目录不是空路径时才检查和创建
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        SemanticDiffError::IoError(std::io::Error::new(
                            e.kind(),
                            format!(
                                "Failed to create output directory {}: {}",
                                parent.display(),
                                e
                            ),
                        ))
                    })?;
                }
            }
        }

        Ok(())
    }
}
