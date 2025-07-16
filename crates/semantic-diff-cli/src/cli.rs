//! 命令行接口模块
//!
//! 提供命令行参数解析和用户交互功能

use clap::{Parser, ValueEnum};
use semantic_diff_core::{HighlightStyle, OutputFormat, Result, SemanticDiffError};

/// semantic-diff 命令行工具
#[derive(Parser)]
#[command(name = "semantic-diff")]
#[command(about = "A semantic code diff tool for Go projects")]
#[command(version = "0.1.0")]
pub struct Cli {
    /// Git 提交哈希
    #[arg(help = "Git commit hash to analyze")]
    pub commit_hash: String,

    /// 输出格式
    #[arg(short, long, value_enum, default_value_t = OutputFormatArg::PlainText)]
    pub format: OutputFormatArg,

    /// 高亮样式
    #[arg(long, value_enum, default_value_t = HighlightStyleArg::Inline)]
    pub highlight: HighlightStyleArg,

    /// 仓库路径
    #[arg(short, long, default_value = ".")]
    pub repo_path: String,

    /// 是否包含注释
    #[arg(long, default_value_t = true)]
    pub include_comments: bool,

    /// 最大依赖深度
    #[arg(long, default_value_t = 3)]
    pub max_depth: u32,

    /// 排除测试文件
    #[arg(long, default_value_t = false)]
    pub exclude_tests: bool,

    /// 详细输出
    #[arg(short, long)]
    pub verbose: bool,
}

/// 输出格式命令行参数
#[derive(Clone, ValueEnum)]
pub enum OutputFormatArg {
    PlainText,
    Markdown,
    Html,
}

/// 高亮样式命令行参数
#[derive(Clone, ValueEnum)]
pub enum HighlightStyleArg {
    None,
    Inline,
    Separate,
}

/// 配置信息
pub struct Config {
    pub output_format: OutputFormat,
    pub include_comments: bool,
    pub max_dependency_depth: u32,
    pub exclude_test_files: bool,
    pub highlight_style: HighlightStyle,
    pub repo_path: String,
    pub verbose: bool,
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
            output_format: cli.format.into(),
            include_comments: cli.include_comments,
            max_dependency_depth: cli.max_depth,
            exclude_test_files: cli.exclude_tests,
            highlight_style: cli.highlight.into(),
            repo_path: cli.repo_path,
            verbose: cli.verbose,
        }
    }
}

impl Cli {
    /// 解析命令行参数
    pub fn parse_args() -> Self {
        // TODO: 在任务 10 中实现完整功能
        Self::parse()
    }

    /// 验证参数
    pub fn validate(&self) -> Result<()> {
        // TODO: 在任务 10 中实现
        if self.commit_hash.is_empty() {
            return Err(SemanticDiffError::InvalidCommitHash(
                "Commit hash cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}
