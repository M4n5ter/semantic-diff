//! 性能优化模块
//!
//! 提供内存高效的 AST 处理、解析器缓存、并发处理等性能优化功能

use crate::error::{Result, SemanticDiffError};
use crate::parser::{LanguageParser, ParserFactory, SourceFile, SupportedLanguage};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// 解析器缓存
///
/// 缓存已创建的解析器实例，避免重复创建的开销
pub struct ParserCache {
    /// 缓存的解析器实例
    /// 注意：由于 tree-sitter 解析器不是 Clone，我们使用工厂函数
    cache: Arc<RwLock<HashMap<SupportedLanguage, Arc<Mutex<Box<dyn LanguageParser>>>>>>,
    /// 缓存统计信息
    stats: Arc<Mutex<CacheStats>>,
}

/// 缓存统计信息
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// 缓存命中次数
    pub hits: u64,
    /// 缓存未命中次数
    pub misses: u64,
    /// 缓存创建次数
    pub creates: u64,
}

impl CacheStats {
    /// 获取缓存命中率
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// 内存高效的 AST 处理器
///
/// 提供内存优化的 AST 处理策略，包括延迟加载、内存池等
pub struct MemoryEfficientAstProcessor {
    /// 最大同时处理的文件数量
    max_concurrent_files: usize,
    /// 内存使用阈值（字节）
    memory_threshold: usize,
    /// 是否启用内存监控
    memory_monitoring: bool,
}

/// 并发文件处理器
///
/// 使用 rayon 实现高效的并发文件处理
pub struct ConcurrentFileProcessor {
    /// 线程池大小
    thread_pool_size: usize,
    /// 批处理大小
    batch_size: usize,
    /// 解析器缓存
    parser_cache: Arc<ParserCache>,
    /// AST 处理器
    ast_processor: MemoryEfficientAstProcessor,
}

/// 性能监控器
///
/// 监控解析性能和资源使用情况
pub struct PerformanceMonitor {
    /// 开始时间
    start_time: Instant,
    /// 处理的文件数量
    files_processed: Arc<Mutex<u64>>,
    /// 总处理时间
    total_processing_time: Arc<Mutex<Duration>>,
    /// 错误计数
    error_count: Arc<Mutex<u64>>,
}

/// 解析结果
#[derive(Debug)]
pub struct ParseResult {
    /// 解析成功的文件
    pub successful: Vec<SourceFile>,
    /// 解析失败的文件及错误信息
    pub failed: Vec<(PathBuf, SemanticDiffError)>,
    /// 性能统计
    pub performance_stats: PerformanceStats,
}

/// 性能统计信息
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    /// 总处理时间
    pub total_duration: Duration,
    /// 处理的文件数量
    pub files_processed: u64,
    /// 成功处理的文件数量
    pub successful_files: u64,
    /// 失败的文件数量
    pub failed_files: u64,
    /// 平均每个文件的处理时间
    pub avg_file_processing_time: Duration,
    /// 内存使用峰值（如果启用监控）
    pub peak_memory_usage: Option<usize>,
    /// 缓存统计
    pub cache_stats: CacheStats,
}

impl Default for ParserCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ParserCache {
    /// 创建新的解析器缓存
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(Mutex::new(CacheStats::default())),
        }
    }

    /// 获取或创建解析器
    pub fn get_or_create_parser(
        &self,
        language: SupportedLanguage,
    ) -> Result<Arc<Mutex<Box<dyn LanguageParser>>>> {
        // 首先尝试从缓存中获取
        {
            let cache = self.cache.read().unwrap();
            if let Some(parser) = cache.get(&language) {
                // 缓存命中
                let mut stats = self.stats.lock().unwrap();
                stats.hits += 1;
                debug!("Parser cache hit for language: {:?}", language);
                return Ok(parser.clone());
            }
        }

        // 缓存未命中，创建新的解析器
        let mut stats = self.stats.lock().unwrap();
        stats.misses += 1;
        stats.creates += 1;
        drop(stats);

        debug!(
            "Parser cache miss for language: {:?}, creating new parser",
            language
        );
        let parser = ParserFactory::create_parser(language)?;
        let parser_arc = Arc::new(Mutex::new(parser));

        // 将新创建的解析器添加到缓存
        {
            let mut cache = self.cache.write().unwrap();
            cache.insert(language, parser_arc.clone());
        }

        Ok(parser_arc)
    }

    /// 获取缓存统计信息
    pub fn get_stats(&self) -> CacheStats {
        self.stats.lock().unwrap().clone()
    }

    /// 清空缓存
    pub fn clear(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
        debug!("Parser cache cleared");
    }

    /// 获取缓存大小
    pub fn size(&self) -> usize {
        let cache = self.cache.read().unwrap();
        cache.len()
    }
}

impl Default for MemoryEfficientAstProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryEfficientAstProcessor {
    /// 创建新的内存高效 AST 处理器
    pub fn new() -> Self {
        Self {
            max_concurrent_files: num_cpus::get() * 2,
            memory_threshold: 512 * 1024 * 1024, // 512MB
            memory_monitoring: true,
        }
    }

    /// 设置最大并发文件数
    pub fn with_max_concurrent_files(mut self, max_files: usize) -> Self {
        self.max_concurrent_files = max_files;
        self
    }

    /// 设置内存阈值
    pub fn with_memory_threshold(mut self, threshold: usize) -> Self {
        self.memory_threshold = threshold;
        self
    }

    /// 启用或禁用内存监控
    pub fn with_memory_monitoring(mut self, enabled: bool) -> Self {
        self.memory_monitoring = enabled;
        self
    }

    /// 检查内存使用情况
    pub fn check_memory_usage(&self) -> Option<usize> {
        if !self.memory_monitoring {
            return None;
        }

        // 在实际实现中，这里应该使用系统调用获取内存使用情况
        // 这里提供一个简化的实现
        #[cfg(target_os = "linux")]
        {
            self.get_memory_usage_linux()
        }
        #[cfg(not(target_os = "linux"))]
        {
            // 对于非 Linux 系统，返回 None 或使用其他方法
            None
        }
    }

    #[cfg(target_os = "linux")]
    fn get_memory_usage_linux(&self) -> Option<usize> {
        use std::fs;

        // 读取 /proc/self/status 获取内存使用情况
        if let Ok(status) = fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(memory_str) = line.split_whitespace().nth(1) {
                        if let Ok(memory_kb) = memory_str.parse::<usize>() {
                            return Some(memory_kb * 1024); // 转换为字节
                        }
                    }
                }
            }
        }
        None
    }

    /// 检查是否应该触发内存清理
    pub fn should_trigger_gc(&self) -> bool {
        if let Some(current_memory) = self.check_memory_usage() {
            current_memory > self.memory_threshold
        } else {
            false
        }
    }

    /// 触发内存清理
    pub fn trigger_gc(&self) {
        if self.should_trigger_gc() {
            debug!("Triggering garbage collection due to high memory usage");
            // 在 Rust 中，我们不能直接触发 GC，但可以释放一些缓存
            // 这里可以清理一些缓存或者建议系统进行内存整理
        }
    }
}

impl ConcurrentFileProcessor {
    /// 创建新的并发文件处理器
    pub fn new() -> Self {
        let thread_pool_size = num_cpus::get();
        Self {
            thread_pool_size,
            batch_size: 10,
            parser_cache: Arc::new(ParserCache::new()),
            ast_processor: MemoryEfficientAstProcessor::new(),
        }
    }

    /// 设置线程池大小
    pub fn with_thread_pool_size(mut self, size: usize) -> Self {
        self.thread_pool_size = size;
        self
    }

    /// 设置批处理大小
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// 设置解析器缓存
    pub fn with_parser_cache(mut self, cache: Arc<ParserCache>) -> Self {
        self.parser_cache = cache;
        self
    }

    /// 设置 AST 处理器
    pub fn with_ast_processor(mut self, processor: MemoryEfficientAstProcessor) -> Self {
        self.ast_processor = processor;
        self
    }

    /// 并发处理多个文件
    pub fn process_files_concurrent(&self, file_paths: &[PathBuf]) -> Result<ParseResult> {
        let monitor = PerformanceMonitor::new();

        info!("开始并发处理 {} 个文件", file_paths.len());

        // 配置 rayon 线程池
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.thread_pool_size)
            .build()
            .map_err(|e| {
                SemanticDiffError::ParseError(format!("Failed to create thread pool: {e}"))
            })?;

        // 使用线程池处理文件
        let results: Vec<_> = pool.install(|| {
            file_paths
                .par_chunks(self.batch_size)
                .flat_map(|chunk| {
                    chunk.par_iter().map(|file_path| {
                        self.process_single_file_with_recovery(file_path, &monitor)
                    })
                })
                .collect()
        });

        // 分离成功和失败的结果
        let mut successful = Vec::new();
        let mut failed = Vec::new();

        for result in results {
            match result {
                Ok(source_file) => successful.push(source_file),
                Err((path, error)) => failed.push((path, error)),
            }
        }

        let performance_stats = monitor.get_stats(self.parser_cache.get_stats());

        info!(
            "文件处理完成: 成功 {}, 失败 {}, 总耗时 {:?}",
            successful.len(),
            failed.len(),
            performance_stats.total_duration
        );

        Ok(ParseResult {
            successful,
            failed,
            performance_stats,
        })
    }

    /// 处理单个文件，包含错误恢复机制
    fn process_single_file_with_recovery(
        &self,
        file_path: &Path,
        monitor: &PerformanceMonitor,
    ) -> std::result::Result<SourceFile, (PathBuf, SemanticDiffError)> {
        let start_time = Instant::now();

        // 检查内存使用情况
        if self.ast_processor.should_trigger_gc() {
            self.ast_processor.trigger_gc();
        }

        let result = self.process_single_file(file_path);

        let processing_time = start_time.elapsed();
        monitor.record_file_processed(processing_time);

        match result {
            Ok(source_file) => {
                debug!("成功处理文件: {:?}, 耗时: {:?}", file_path, processing_time);
                Ok(source_file)
            }
            Err(error) => {
                warn!(
                    "处理文件失败: {:?}, 错误: {}, 耗时: {:?}",
                    file_path, error, processing_time
                );
                monitor.record_error();
                Err((file_path.to_path_buf(), error))
            }
        }
    }

    /// 处理单个文件
    fn process_single_file(&self, file_path: &Path) -> Result<SourceFile> {
        // 检测语言类型
        let language = ParserFactory::detect_language(file_path).ok_or_else(|| {
            SemanticDiffError::UnsupportedFileType(file_path.to_string_lossy().to_string())
        })?;

        // 从缓存获取解析器
        let parser_arc = self.parser_cache.get_or_create_parser(language)?;

        // 读取文件内容
        let source_code = std::fs::read_to_string(file_path).map_err(|e| {
            SemanticDiffError::IoError(std::io::Error::new(
                e.kind(),
                format!("Failed to read file {}: {}", file_path.display(), e),
            ))
        })?;

        // 解析文件
        let syntax_tree = {
            let mut parser = parser_arc.lock().unwrap();
            parser.parse_source(&source_code)?
        };

        // 提取语言特定信息
        let language_specific =
            self.extract_language_specific_info(&syntax_tree, &source_code, file_path, language)?;

        Ok(SourceFile {
            path: file_path.to_path_buf(),
            source_code,
            syntax_tree,
            language,
            language_specific,
        })
    }

    /// 提取语言特定信息
    fn extract_language_specific_info(
        &self,
        syntax_tree: &tree_sitter::Tree,
        source_code: &str,
        file_path: &Path,
        language: SupportedLanguage,
    ) -> Result<Box<dyn crate::parser::LanguageSpecificInfo>> {
        match language {
            SupportedLanguage::Go => {
                self.extract_go_specific_info(syntax_tree, source_code, file_path)
            } // 未来可以在这里添加其他语言的支持
        }
    }

    /// 提取 Go 语言特定信息
    fn extract_go_specific_info(
        &self,
        syntax_tree: &tree_sitter::Tree,
        source_code: &str,
        file_path: &Path,
    ) -> Result<Box<dyn crate::parser::LanguageSpecificInfo>> {
        use crate::parser::{GoLanguageInfo, common::CstNavigator};

        let navigator = CstNavigator::new();
        let root = syntax_tree.root_node();

        // 提取包名
        let package_name = self.extract_package_name(root, source_code, &navigator);

        // 提取导入
        let imports = self.extract_imports(root, source_code, &navigator);

        // 提取声明
        let declarations =
            self.extract_go_declarations(root, source_code, file_path, &navigator)?;

        Ok(Box::new(GoLanguageInfo {
            package_name,
            imports,
            declarations,
        }))
    }

    /// 提取 Go 包名
    fn extract_package_name(
        &self,
        root: tree_sitter::Node,
        source_code: &str,
        _navigator: &crate::parser::common::CstNavigator,
    ) -> String {
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if child.kind() == "package_clause" {
                // 查找包名标识符
                let mut pkg_cursor = child.walk();
                for pkg_child in child.children(&mut pkg_cursor) {
                    if pkg_child.kind() == "package_identifier" {
                        return source_code[pkg_child.byte_range()].to_string();
                    }
                }
            }
        }
        "main".to_string() // 默认包名
    }

    /// 提取导入声明
    fn extract_imports(
        &self,
        root: tree_sitter::Node,
        source_code: &str,
        navigator: &crate::parser::common::CstNavigator,
    ) -> Vec<crate::parser::Import> {
        let mut imports = Vec::new();
        let import_nodes = navigator.find_import_declarations(root);

        for import_node in import_nodes {
            let mut cursor = import_node.walk();
            for child in import_node.children(&mut cursor) {
                match child.kind() {
                    "import_spec" => {
                        if let Some(import) = self.extract_single_import(child, source_code) {
                            imports.push(import);
                        }
                    }
                    "import_spec_list" => {
                        let mut spec_cursor = child.walk();
                        for spec_child in child.children(&mut spec_cursor) {
                            if spec_child.kind() == "import_spec" {
                                if let Some(import) =
                                    self.extract_single_import(spec_child, source_code)
                                {
                                    imports.push(import);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        imports
    }

    /// 提取单个导入
    fn extract_single_import(
        &self,
        import_spec: tree_sitter::Node,
        source_code: &str,
    ) -> Option<crate::parser::Import> {
        let mut path = String::new();
        let mut alias = None;

        let mut cursor = import_spec.walk();
        for child in import_spec.children(&mut cursor) {
            match child.kind() {
                "interpreted_string_literal" => {
                    let path_str = source_code[child.byte_range()].to_string();
                    path = path_str.trim_matches('"').to_string();
                }
                "package_identifier" => {
                    alias = Some(source_code[child.byte_range()].to_string());
                }
                _ => {}
            }
        }

        if !path.is_empty() {
            Some(crate::parser::Import { path, alias })
        } else {
            None
        }
    }

    /// 提取 Go 声明
    fn extract_go_declarations(
        &self,
        root: tree_sitter::Node,
        source_code: &str,
        file_path: &Path,
        navigator: &crate::parser::common::CstNavigator,
    ) -> Result<Vec<Box<dyn crate::parser::common::Declaration>>> {
        let mut declarations = Vec::new();

        // 提取函数声明
        let function_nodes = navigator.find_function_declarations(root);
        for func_node in function_nodes {
            if let Ok(func_info) =
                self.extract_function_info(func_node, source_code, file_path, navigator)
            {
                declarations.push(Box::new(crate::parser::GoDeclaration::Function(func_info))
                    as Box<dyn crate::parser::common::Declaration>);
            }
        }

        // 提取类型声明
        let type_nodes = navigator.find_type_declarations(root);
        for type_node in type_nodes {
            if let Ok(type_def) = self.extract_type_definition(type_node, source_code, file_path) {
                declarations.push(Box::new(crate::parser::GoDeclaration::Type(type_def))
                    as Box<dyn crate::parser::common::Declaration>);
            }
        }

        Ok(declarations)
    }

    /// 提取函数信息
    fn extract_function_info(
        &self,
        func_node: tree_sitter::Node,
        source_code: &str,
        file_path: &Path,
        navigator: &crate::parser::common::CstNavigator,
    ) -> Result<crate::parser::GoFunctionInfo> {
        use crate::parser::{GoParameter, GoReceiverInfo, GoType};

        // 获取函数签名
        let signature = navigator
            .get_function_signature(func_node, source_code)
            .ok_or_else(|| {
                SemanticDiffError::ParseError("Failed to extract function signature".to_string())
            })?;

        // 获取函数体
        let body = if let Some(body_node) = navigator.get_function_body(func_node) {
            source_code[body_node.byte_range()].to_string()
        } else {
            String::new()
        };

        // 获取行号范围
        let (start_line, end_line) = navigator.get_node_line_range(func_node);

        // 转换参数
        let parameters: Vec<GoParameter> = signature
            .parameters
            .iter()
            .enumerate()
            .map(|(i, param_str)| GoParameter {
                name: format!("param{i}"),
                param_type: GoType {
                    name: param_str.clone(),
                    is_pointer: param_str.contains('*'),
                    is_slice: param_str.contains("[]"),
                },
            })
            .collect();

        // 转换返回类型
        let return_types: Vec<GoType> = signature
            .return_types
            .iter()
            .map(|ret_str| GoType {
                name: ret_str.clone(),
                is_pointer: ret_str.contains('*'),
                is_slice: ret_str.contains("[]"),
            })
            .collect();

        // 转换接收者信息
        let receiver = signature.receiver.map(|recv_str| {
            let is_pointer = recv_str.contains('*');
            let type_name = recv_str.replace(['*', '(', ')'], "").trim().to_string();

            GoReceiverInfo {
                name: "self".to_string(),
                type_name,
                is_pointer,
            }
        });

        Ok(crate::parser::GoFunctionInfo {
            name: signature.name.clone(),
            receiver,
            parameters,
            return_types,
            body,
            start_line,
            end_line,
            file_path: file_path.to_path_buf(),
        })
    }

    /// 提取类型定义
    fn extract_type_definition(
        &self,
        type_node: tree_sitter::Node,
        source_code: &str,
        file_path: &Path,
    ) -> Result<crate::parser::GoTypeDefinition> {
        use crate::parser::{GoTypeDefinition, GoTypeKind};

        let definition = source_code[type_node.byte_range()].to_string();

        // 简化的类型名称提取
        let name = if let Some(name_node) = type_node.child_by_field_name("name") {
            source_code[name_node.byte_range()].to_string()
        } else {
            "UnknownType".to_string()
        };

        // 简化的类型种类判断
        let kind = if definition.contains("struct") {
            GoTypeKind::Struct
        } else if definition.contains("interface") {
            GoTypeKind::Interface
        } else {
            GoTypeKind::Alias
        };

        Ok(GoTypeDefinition {
            name,
            kind,
            definition,
            file_path: file_path.to_path_buf(),
            dependencies: Vec::new(), // 这里可以进一步分析依赖
        })
    }
}

impl Default for ConcurrentFileProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceMonitor {
    /// 创建新的性能监控器
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            files_processed: Arc::new(Mutex::new(0)),
            total_processing_time: Arc::new(Mutex::new(Duration::ZERO)),
            error_count: Arc::new(Mutex::new(0)),
        }
    }

    /// 记录文件处理完成
    pub fn record_file_processed(&self, processing_time: Duration) {
        let mut files = self.files_processed.lock().unwrap();
        *files += 1;

        let mut total_time = self.total_processing_time.lock().unwrap();
        *total_time += processing_time;
    }

    /// 记录错误
    pub fn record_error(&self) {
        let mut errors = self.error_count.lock().unwrap();
        *errors += 1;
    }

    /// 获取性能统计信息
    pub fn get_stats(&self, cache_stats: CacheStats) -> PerformanceStats {
        let total_duration = self.start_time.elapsed();
        let files_processed = *self.files_processed.lock().unwrap();
        let error_count = *self.error_count.lock().unwrap();
        let successful_files = files_processed - error_count;

        let avg_file_processing_time = if files_processed > 0 {
            *self.total_processing_time.lock().unwrap() / files_processed as u32
        } else {
            Duration::ZERO
        };

        PerformanceStats {
            total_duration,
            files_processed,
            successful_files,
            failed_files: error_count,
            avg_file_processing_time,
            peak_memory_usage: None, // 可以在实际实现中添加内存监控
            cache_stats,
        }
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// 错误恢复策略
pub struct ErrorRecoveryStrategy {
    /// 最大重试次数
    max_retries: usize,
    /// 重试延迟
    retry_delay: Duration,
    /// 是否跳过损坏的文件
    skip_corrupted_files: bool,
}

impl Default for ErrorRecoveryStrategy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay: Duration::from_millis(100),
            skip_corrupted_files: true,
        }
    }
}

impl ErrorRecoveryStrategy {
    /// 创建新的错误恢复策略
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置最大重试次数
    pub fn with_max_retries(mut self, retries: usize) -> Self {
        self.max_retries = retries;
        self
    }

    /// 设置重试延迟
    pub fn with_retry_delay(mut self, delay: Duration) -> Self {
        self.retry_delay = delay;
        self
    }

    /// 设置是否跳过损坏的文件
    pub fn with_skip_corrupted_files(mut self, skip: bool) -> Self {
        self.skip_corrupted_files = skip;
        self
    }

    /// 执行带重试的操作
    pub fn execute_with_retry<T, F>(&self, mut operation: F) -> Result<T>
    where
        F: FnMut() -> Result<T>,
    {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            match operation() {
                Ok(result) => return Ok(result),
                Err(error) => {
                    last_error = Some(error);

                    if attempt < self.max_retries {
                        debug!("操作失败，第 {} 次重试", attempt + 1);
                        std::thread::sleep(self.retry_delay);
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }

    /// 判断错误是否可恢复
    pub fn is_recoverable_error(&self, error: &SemanticDiffError) -> bool {
        match error {
            SemanticDiffError::IoError(_) => true,
            SemanticDiffError::ParseError(_) => self.skip_corrupted_files,
            SemanticDiffError::TreeSitterError(_) => self.skip_corrupted_files,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_cache() {
        let cache = ParserCache::new();

        // 测试缓存未命中
        let _parser1 = cache.get_or_create_parser(SupportedLanguage::Go).unwrap();
        let stats = cache.get_stats();
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.creates, 1);

        // 测试缓存命中
        let _parser2 = cache.get_or_create_parser(SupportedLanguage::Go).unwrap();
        let stats = cache.get_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.creates, 1);
    }

    #[test]
    fn test_memory_efficient_ast_processor() {
        let processor = MemoryEfficientAstProcessor::new()
            .with_max_concurrent_files(4)
            .with_memory_threshold(1024 * 1024)
            .with_memory_monitoring(false);

        // 测试基本配置
        assert_eq!(processor.max_concurrent_files, 4);
        assert_eq!(processor.memory_threshold, 1024 * 1024);
        assert!(!processor.memory_monitoring);
    }

    #[test]
    fn test_concurrent_file_processor() {
        let processor = ConcurrentFileProcessor::new()
            .with_thread_pool_size(2)
            .with_batch_size(5);

        assert_eq!(processor.thread_pool_size, 2);
        assert_eq!(processor.batch_size, 5);
    }

    #[test]
    fn test_error_recovery_strategy() {
        let strategy = ErrorRecoveryStrategy::new()
            .with_max_retries(2)
            .with_retry_delay(Duration::from_millis(50));

        assert_eq!(strategy.max_retries, 2);
        assert_eq!(strategy.retry_delay, Duration::from_millis(50));

        // 测试可恢复错误判断
        let io_error = SemanticDiffError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "File not found",
        ));
        assert!(strategy.is_recoverable_error(&io_error));
    }

    #[test]
    fn test_performance_monitor() {
        let monitor = PerformanceMonitor::new();

        // 模拟处理文件
        monitor.record_file_processed(Duration::from_millis(100));
        monitor.record_file_processed(Duration::from_millis(200));
        monitor.record_error();

        let stats = monitor.get_stats(CacheStats::default());
        assert_eq!(stats.files_processed, 2);
        assert_eq!(stats.failed_files, 1);
        assert_eq!(stats.successful_files, 1);
    }
}
