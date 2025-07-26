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
    // 验证和加载配置
    validate_and_load_config(&config)?;

    info!(
        "Analyzing commit {} in repository: {}",
        config.commit_hash,
        config.repo_path.display()
    );

    // 1. 初始化 Git 解析器
    debug!("Initializing Git diff parser");
    let git_parser = semantic_diff_core::GitDiffParser::new(config.repo_path.clone())?;

    // 2. 解析提交差异
    debug!("Parsing commit diff for: {}", config.commit_hash);
    let file_changes = git_parser.parse_commit(&config.commit_hash)?;

    if file_changes.is_empty() {
        info!("No file changes found in commit {}", config.commit_hash);
        let output = "No changes found in the specified commit.\n";
        write_output(output, &config)?;
        return Ok(());
    }

    info!("Found {} changed files", file_changes.len());

    // 3. 分析变更的文件
    debug!("Analyzing changed files");
    let analyzed_files = analyze_changed_files(&file_changes, &config)?;

    if analyzed_files.is_empty() {
        info!("No supported files found in changes");
        let output = "No supported source files found in the changes.\n";
        write_output(output, &config)?;
        return Ok(());
    }

    info!(
        "Successfully analyzed {} source files",
        analyzed_files.len()
    );

    // 4. 查找变更的函数和其他目标
    debug!("Finding changed targets");
    let change_targets = find_change_targets(&file_changes, &analyzed_files, &config)?;

    if change_targets.is_empty() {
        info!("No change targets found");
        let output = "No functions or types were changed in the specified commit.\n";
        write_output(output, &config)?;
        return Ok(());
    }

    info!("Found {} change targets", change_targets.len());

    // 5. 初始化语义上下文提取器
    debug!("Initializing semantic context extractor");
    let context_extractor = create_context_extractor(&config)?;

    // 6. 提取语义上下文
    debug!("Extracting semantic context");
    let semantic_contexts = extract_semantic_contexts(
        &change_targets,
        &analyzed_files,
        &context_extractor,
        &config,
    )?;

    info!(
        "Extracted semantic context for {} targets",
        semantic_contexts.len()
    );

    // 7. 初始化代码生成器
    debug!("Initializing code slice generator");
    let code_generator = create_code_generator(&config);

    // 8. 生成代码切片
    debug!("Generating code slices");
    let code_slices = generate_code_slices(&semantic_contexts, &file_changes, &code_generator)?;

    info!("Generated {} code slices", code_slices.len());

    // 9. 格式化并输出结果
    debug!("Formatting and outputting results");
    format_and_output(&code_slices, &config)?;

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

/// 分析变更的文件
fn analyze_changed_files(
    file_changes: &[semantic_diff_core::FileChange],
    config: &Config,
) -> Result<Vec<semantic_diff_core::SourceFile>> {
    use semantic_diff_core::analyzer::SourceAnalyzer;

    let mut analyzed_files = Vec::new();

    for file_change in file_changes {
        // 跳过二进制文件
        if file_change.is_binary {
            debug!("Skipping binary file: {:?}", file_change.file_path);
            continue;
        }

        // 跳过删除的文件
        if matches!(
            file_change.change_type,
            semantic_diff_core::ChangeType::Deleted
        ) {
            debug!("Skipping deleted file: {:?}", file_change.file_path);
            continue;
        }

        // 检查是否应该排除测试文件
        if config.exclude_test_files && is_test_file(&file_change.file_path) {
            debug!("Skipping test file: {:?}", file_change.file_path);
            continue;
        }

        // 构建完整的文件路径
        let full_path = config.repo_path.join(&file_change.file_path);

        // 检查文件是否存在
        if !full_path.exists() {
            debug!("File does not exist: {:?}", full_path);
            continue;
        }

        // 尝试分析文件
        match SourceAnalyzer::new_for_file(&full_path) {
            Ok(mut analyzer) => {
                match analyzer.analyze_file(&full_path) {
                    Ok(source_file) => {
                        debug!("Successfully analyzed file: {:?}", file_change.file_path);
                        analyzed_files.push(source_file);
                    }
                    Err(e) => {
                        warn!("Failed to analyze file {:?}: {}", file_change.file_path, e);
                        // 继续处理其他文件
                    }
                }
            }
            Err(e) => {
                debug!("Unsupported file type {:?}: {}", file_change.file_path, e);
                // 继续处理其他文件
            }
        }
    }

    Ok(analyzed_files)
}

/// 检查是否为测试文件
fn is_test_file(file_path: &std::path::Path) -> bool {
    let file_name = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");

    // Go 测试文件通常以 _test.go 结尾
    file_name.ends_with("_test.go") ||
    // 或者在 test 目录中
    file_path.components().any(|component| {
        component.as_os_str().to_str().unwrap_or("") == "test" ||
        component.as_os_str().to_str().unwrap_or("") == "tests"
    })
}

/// 查找变更目标
fn find_change_targets(
    file_changes: &[semantic_diff_core::FileChange],
    analyzed_files: &[semantic_diff_core::SourceFile],
    config: &Config,
) -> Result<Vec<semantic_diff_core::extractor::ChangeTarget>> {
    use semantic_diff_core::{analyzer::SourceAnalyzer, extractor::ChangeTarget};

    let mut change_targets = Vec::new();

    // 为每个分析的文件查找变更的函数
    for source_file in analyzed_files {
        // 找到对应的文件变更
        let file_change = file_changes
            .iter()
            .find(|fc| config.repo_path.join(&fc.file_path) == source_file.path);

        if let Some(file_change) = file_change {
            // 创建源分析器
            let analyzer = SourceAnalyzer::new_for_language(source_file.language)?;

            // 查找变更的函数
            let changed_functions =
                analyzer.find_changed_functions(source_file, &file_change.hunks)?;

            for function in changed_functions {
                debug!(
                    "Found changed function: {} in {:?}",
                    function.name, source_file.path
                );
                change_targets.push(ChangeTarget::Function(function));
            }

            // 如果不是只显示函数，还要查找其他类型的变更
            if !config.functions_only {
                // TODO: 在未来的版本中添加对类型、变量、常量变更的检测
                // 目前主要关注函数变更
            }
        }
    }

    Ok(change_targets)
}

/// 创建语义上下文提取器
fn create_context_extractor(
    config: &Config,
) -> Result<semantic_diff_core::SemanticContextExtractor> {
    // 尝试从项目根目录创建提取器（以获取 go.mod 信息）
    match semantic_diff_core::SemanticContextExtractor::from_project_root(&config.repo_path) {
        Ok(extractor) => {
            debug!("Created context extractor with project module information");
            Ok(extractor.with_max_recursion_depth(config.max_dependency_depth as usize))
        }
        Err(e) => {
            debug!(
                "Failed to read project module info: {}, using default extractor",
                e
            );
            Ok(semantic_diff_core::SemanticContextExtractor::new()
                .with_max_recursion_depth(config.max_dependency_depth as usize))
        }
    }
}

/// 提取语义上下文
fn extract_semantic_contexts(
    change_targets: &[semantic_diff_core::extractor::ChangeTarget],
    analyzed_files: &[semantic_diff_core::SourceFile],
    context_extractor: &semantic_diff_core::SemanticContextExtractor,
    _config: &Config,
) -> Result<Vec<semantic_diff_core::SemanticContext>> {
    if change_targets.len() > 10 {
        // 对于大量目标，使用批量处理
        info!(
            "Using batch processing for {} targets",
            change_targets.len()
        );
        context_extractor.extract_contexts_in_batches(change_targets, analyzed_files, 5)
    } else if change_targets.len() > 1 {
        // 对于中等数量的目标，使用并发处理
        debug!(
            "Using concurrent processing for {} targets",
            change_targets.len()
        );
        context_extractor.extract_contexts_concurrent(change_targets, analyzed_files)
    } else {
        // 对于单个目标，直接处理
        debug!("Processing single target");
        let mut contexts = Vec::new();
        for target in change_targets {
            let context =
                context_extractor.extract_context_for_target(target.clone(), analyzed_files)?;
            contexts.push(context);
        }
        Ok(contexts)
    }
}

/// 创建代码生成器
fn create_code_generator(config: &Config) -> semantic_diff_core::CodeSliceGenerator {
    use semantic_diff_core::{CodeSliceGenerator, generator::GeneratorConfig};

    let generator_config = GeneratorConfig {
        include_comments: config.include_comments,
        include_imports: true,
        include_types: true,
        include_dependent_functions: !config.functions_only,
        max_lines: config.max_lines.map(|n| n as usize),
        output_format: config.output_format.clone(),
        highlight_style: config.highlight_style.clone(),
    };

    CodeSliceGenerator::with_config(generator_config)
}

/// 生成代码切片
fn generate_code_slices(
    semantic_contexts: &[semantic_diff_core::SemanticContext],
    file_changes: &[semantic_diff_core::FileChange],
    code_generator: &semantic_diff_core::CodeSliceGenerator,
) -> Result<Vec<semantic_diff_core::CodeSlice>> {
    let mut code_slices = Vec::new();

    for context in semantic_contexts {
        // 找到对应的文件变更以获取差异信息
        let target_file_path = context.change_target.file_path();
        let relevant_hunks: Vec<_> = file_changes
            .iter()
            .filter(|fc| fc.file_path == target_file_path.file_name().unwrap_or_default())
            .flat_map(|fc| &fc.hunks)
            .cloned()
            .collect();

        // 生成代码切片
        match code_generator.generate_slice(context, &relevant_hunks) {
            Ok(slice) => {
                debug!(
                    "Generated code slice for target: {}",
                    context.change_target.name()
                );
                code_slices.push(slice);
            }
            Err(e) => {
                warn!(
                    "Failed to generate code slice for {}: {}",
                    context.change_target.name(),
                    e
                );
                // 继续处理其他上下文
            }
        }
    }

    Ok(code_slices)
}

/// 格式化并输出结果
fn format_and_output(code_slices: &[semantic_diff_core::CodeSlice], config: &Config) -> Result<()> {
    use semantic_diff_core::{formatter::OutputRenderer, generator::CodeFormatter};

    if code_slices.is_empty() {
        let output = "No code slices generated.\n";
        write_output(output, config)?;
        return Ok(());
    }

    // 创建格式化器（暂时未使用，但保留以备将来使用）
    let _formatter =
        CodeFormatter::new(config.output_format.clone(), config.highlight_style.clone());

    // 创建输出渲染器配置
    let renderer_config = semantic_diff_core::formatter::FormatterConfig {
        output_format: config.output_format.clone(),
        highlight_style: config.highlight_style.clone(),
        show_line_numbers: config.verbose,
        show_file_paths: true,
        show_statistics: config.verbose,
        enable_colors: true,
        block_title_style: semantic_diff_core::formatter::BlockTitleStyle::Detailed,
        custom_css: None,
        max_line_width: config.max_lines.map(|n| n as usize),
        indent_size: 4,
    };

    let renderer = OutputRenderer::new(renderer_config);

    // 生成最终输出
    let mut final_output = String::new();

    // 添加总体统计信息
    if config.verbose {
        final_output.push_str(&format!(
            "// Semantic Diff Analysis Results\n\
             // Commit: {}\n\
             // Repository: {}\n\
             // Generated {} code slices\n\n",
            config.commit_hash,
            config.repo_path.display(),
            code_slices.len()
        ));
    }

    // 处理每个代码切片
    for (index, slice) in code_slices.iter().enumerate() {
        if index > 0 {
            final_output.push_str("\n\n");
            final_output.push_str("=".repeat(80).as_str());
            final_output.push_str("\n\n");
        }

        // 使用渲染器处理
        let rendered_output = renderer.render(slice)?;
        final_output.push_str(&rendered_output.content);

        // 添加统计信息（如果启用详细模式）
        if config.verbose {
            let stats = slice.get_stats();
            final_output.push_str(&format!(
                "\n// Slice Statistics:\n\
                 //   Total lines: {}\n\
                 //   Highlighted lines: {}\n\
                 //   Types: {}, Functions: {}, Constants: {}\n\
                 //   Files involved: {}\n",
                stats.total_lines,
                stats.highlighted_lines,
                stats.types_count,
                stats.functions_count,
                stats.constants_count,
                stats.files_count
            ));
        }
    }

    // 输出结果
    write_output(&final_output, config)?;

    Ok(())
}
/// 验证和加载配置
fn validate_and_load_config(config: &Config) -> Result<()> {
    debug!("Validating configuration");

    // 验证仓库路径
    if !config.repo_path.exists() {
        return Err(semantic_diff_core::SemanticDiffError::IoError(
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "Repository path does not exist: {}",
                    config.repo_path.display()
                ),
            ),
        ));
    }

    // 检查是否为Git仓库
    let git_dir = config.repo_path.join(".git");
    if !git_dir.exists() {
        return Err(semantic_diff_core::SemanticDiffError::GitError(format!(
            "Not a Git repository: {}",
            config.repo_path.display()
        )));
    }

    // 验证并创建输出文件路径（如果指定）
    if let Some(output_file) = &config.output_file {
        debug!("Output file specified: {}", output_file.display());
        if let Some(parent) = output_file.parent() {
            debug!("Parent directory: {}", parent.display());
            debug!("Parent is empty: {}", parent.as_os_str().is_empty());
            debug!("Parent exists: {}", parent.exists());

            // 只有当父目录不是空路径时才检查和创建
            if !parent.as_os_str().is_empty() && !parent.exists() {
                debug!("Creating output directory: {}", parent.display());
                std::fs::create_dir_all(parent).map_err(|e| {
                    semantic_diff_core::SemanticDiffError::IoError(std::io::Error::new(
                        e.kind(),
                        format!(
                            "Failed to create output directory {}: {}",
                            parent.display(),
                            e
                        ),
                    ))
                })?;
                info!("Created output directory: {}", parent.display());
            }
        } else {
            debug!("No parent directory for output file");
        }
    }

    // 验证配置参数范围
    if config.max_dependency_depth == 0 {
        return Err(semantic_diff_core::SemanticDiffError::ParseError(
            "Max dependency depth must be greater than 0".to_string(),
        ));
    }

    if let Some(max_lines) = config.max_lines {
        if max_lines == 0 {
            return Err(semantic_diff_core::SemanticDiffError::ParseError(
                "Max lines must be greater than 0".to_string(),
            ));
        }
    }

    debug!("Configuration validation completed successfully");
    Ok(())
}
