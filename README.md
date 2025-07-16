# semantic-diff

**`git diff` 告诉你*什么*被修改了，`semantic-diff` 让你理解*为什么*这样修改。**

`semantic-diff` 是一款面向开发者的下一代代码差异分析工具。它超越了传统 `git diff` 基于文本的比较，通过理解代码的语法结构和语义，为你提供一个与变更相关的、完整的、自包含的逻辑上下文。

## 核心问题 (The Problem)

在进行代码审查（Code Review）或分析历史变更时，我们常常遇到这样的困境：

  * **上下文缺失**：`git diff` 只显示了修改的几行代码。你看到 `- result = a + b` 变成了 `+ result = calculate(a, b)`，但 `calculate` 函数是什么？`a` 和 `b` 是什么类型？这个修改位于哪个函数中，这个函数的职责又是什么？你不得不手动在代码库中跳转、搜索，才能拼凑出完整的逻辑。
  * **审查效率低下**：为了理解一个单行修复，你可能需要花费数分钟甚至更长时间来追溯其依赖关系，这极大地拖慢了审查速度。
  * **隐藏的风险**：由于无法一目了然地看到所有相关代码，审查者可能会忽略由变更引发的潜在副作用，从而引入新的 Bug。

传统的 `diff` 工具只关心“文本”变了，而开发者真正关心的是“逻辑”变了。

## 解决方案 (The Solution: semantic-diff)

`semantic-diff` 通过解析代码的抽象语法树（AST），精准地识别出变更的真正影响范围。当一行代码被修改时，`semantic-diff` 会为你提取一个**语义完整的代码切片（Semantically Complete Code Slice）**，其中包含：

  * ✅ **完整的函数/方法体**：展示被修改代码所在的整个函数，让你立即了解其入口、出口和核心逻辑。
  * ✅ **相关的类型定义**：如果函数中使用了项目内的 `StructA` 或 `EnumB`，`semantic-diff` 会自动将这些类型的定义包含进来。
  * ✅ **依赖的内部函数**：如果函数调用了项目内的另一个函数 `helper_func()`，`helper_func` 的定义也会被一并提取。
  * ✅ **方法接收者**：如果修改的是一个方法（method），其所属的结构体（receiver）以及相关的 `impl` 块也会被完整展示。
  * ✅ **智能的依赖过滤**：它能区分项目内代码和第三方库代码，只提取前者，避免无关的外部库代码干扰你的视线。

最终，`semantic-diff` 输出的是一段可以直接复制、独立编译和理解的代码片段，让代码审查变得前所未有的高效和精确。

## 工作原理示例

假设我们有以下的 Go 代码，并且我们修改了 `UpdateUser` 方法中的一行。

**原始 `git diff` 输出：**

```diff
--- a/user.go
+++ b/user.go
@@ -17,5 +17,5 @@
 func (u *User) UpdateStatus(newStatus Status) {
-  u.Status = newStatus
+  u.Status = newStatus
+  u.UpdatedAt = time.Now()
 }
```

这个 `diff` 无法告诉你 `User` 是什么，`Status` 是什么。

**`semantic-diff` 输出：**

```go
// Extracted by semantic-diff for commit [commit-hash]

// Relevant type definitions
type Status int

const (
    StatusActive   Status = 1
    StatusInactive Status = 2
)

type User struct {
    ID        int
    Name      string
    Status    Status
    CreatedAt time.Time
    UpdatedAt time.Time
}

// Full context of the changed method
func (u *User) UpdateStatus(newStatus Status) {
    u.Status = newStatus
    u.UpdatedAt = time.Now() // <-- changed line highlighted
}
```

通过 `semantic-diff` 的输出，你可以立即理解这次变更的全部上下文，无需任何额外的跳转和搜索。

## 技术栈与未来规划

  * **核心语言**：本项目使用 [**Rust**](https://www.rust-lang.org/) 构建，以保证高性能、内存安全和可靠性。
  * **初始目标语言**：首个版本将支持对 [**Go**](https://go.dev/) 语言的分析。
  * **未来蓝图**：
      * 扩展支持更多语言，如 Rust, Python, TypeScript, Java 等。
      * 与主流 Git 平台（如 GitHub, GitLab）集成，提供更流畅的 Code Review 体验。
      * 开发 IDE 插件（如 VS Code），在编辑器中实时展示语义差异。
      * 探索对非函数作用域（如全局变量、模块级配置）变更的智能上下文提取策略。

`semantic-diff` 的使命是革新代码审查的方式，让每一个开发者都能将精力聚焦于逻辑本身，而非在代码的海洋中迷航。我们相信，更好的工具能够创造更好的代码。

**欢迎关注我们的进展，也欢迎未来的贡献者！**