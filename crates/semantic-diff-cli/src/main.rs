//! semantic-diff - 语义代码差异分析工具
//!
//! 这是一个基于 Tree-sitter 的多语言代码差异分析工具，
//! 能够提供比传统 git diff 更丰富的语义上下文信息。

mod cli;

use cli::{Cli, Config};
use semantic_diff_core::Result;
use std::error::Error;
use std::io::{self, Write};
use tracing::{debug, info, warn};
use tracing_subscriber::{EnvFilter, fmt};

fn main() {
    // 解析命令行参数
    let cli = Cli::parse_args();

    // 初始化日志记录 (根据 verbose 标志调整日志级别)
    init_logging(cli.verbose);

    // 验证参数
    if let Err(e) = cli.validate() {
        eprintln!("Error: {e}");
        eprintln!("Use --help for usage information.");
        std::process::exit(1);
    }

    let config: Config = cli.into();

    if config.verbose {
        info!("Starting semantic-diff analysis");
        debug!("Configuration: {:?}", config);
    }

    // 运行主要逻辑
    match run(config) {
        Ok(()) => {
            if let Err(e) = io::stdout().flush() {
                warn!("Failed to flush stdout: {}", e);
            }
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("Error: {e}");
            if let Some(source) = e.source() {
                eprintln!("Caused by: {source}");
            }
            std::process::exit(1);
        }
    }
}

/// 初始化日志记录系统
fn init_logging(verbose: bool) {
    let filter = if verbose {
        EnvFilter::new("semantic_diff=debug,semantic_diff_core=debug")
    } else {
        EnvFilter::new("semantic_diff=info,semantic_diff_core=warn")
    };

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .init();
}

/// 主要应用逻辑
fn run(config: Config) -> Result<()> {
    info!(
        "Analyzing commit {} in repository: {}",
        config.commit_hash,
        config.repo_path.display()
    );

    // 显示分析开始信息
    if !config.verbose {
        eprintln!("Analyzing commit {}...", config.commit_hash);
    }

    // TODO: 在任务 13-16 中实现完整的工作流程
    // 这里只是基础框架，具体实现将在后续任务中完成

    // 1. 初始化 Git 解析器
    debug!("Initializing Git diff parser");
    // let git_parser = GitDiffParser::new(config.repo_path.clone())?;

    // 2. 解析提交差异
    debug!("Parsing commit diff for: {}", config.commit_hash);
    // let file_changes = git_parser.parse_commit(&config.commit_hash)?;

    // 3. 初始化源码分析器
    debug!("Initializing source analyzer");
    // let source_analyzer = SourceAnalyzer::new()?;

    // 4. 分析变更的文件
    debug!("Analyzing changed files");
    // let analyzed_files = analyze_changed_files(&file_changes, &source_analyzer)?;

    // 5. 初始化语义上下文提取器
    debug!("Initializing semantic context extractor");
    // let context_extractor = SemanticContextExtractor::new();

    // 6. 提取语义上下文
    debug!("Extracting semantic context");
    // let semantic_contexts = extract_semantic_contexts(&analyzed_files, &context_extractor)?;

    // 7. 初始化代码生成器
    debug!("Initializing code slice generator");
    // let code_generator = CodeSliceGenerator::with_config(generator_config_from(&config));

    // 8. 生成代码切片
    debug!("Generating code slices");
    // let code_slices = generate_code_slices(&semantic_contexts, &code_generator)?;

    // 9. 格式化并输出结果
    debug!("Formatting and outputting results");
    // format_and_output(&code_slices, &config)?;

    // 临时输出，显示工具已正确初始化
    let output = format!(
        "semantic-diff analysis initialized successfully!\n\
         Repository: {}\n\
         Commit: {}\n\
         Output format: {:?}\n\
         Highlight style: {:?}\n\
         Max dependency depth: {}\n\
         Include comments: {}\n\
         Exclude tests: {}\n\
         Functions only: {}\n\
         \n\
         Full implementation will be completed in subsequent tasks.",
        config.repo_path.display(),
        config.commit_hash,
        config.output_format,
        config.highlight_style,
        config.max_dependency_depth,
        config.include_comments,
        config.exclude_test_files,
        config.functions_only
    );

    // 输出到文件或标准输出
    write_output(&output, &config)?;

    info!("Analysis completed successfully");
    Ok(())
}

/// 将输出写入文件或标准输出
fn write_output(content: &str, config: &Config) -> Result<()> {
    match &config.output_file {
        Some(file_path) => {
            debug!("Writing output to file: {}", file_path.display());
            std::fs::write(file_path, content)?;
            if !config.verbose {
                eprintln!("Output written to: {}", file_path.display());
            }
        }
        None => {
            print!("{content}");
        }
    }
    Ok(())
}
