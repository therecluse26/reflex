# Dependency Resolution Implementation TODO

**Created:** 2025-11-12
**Status:** Planning Complete - Ready to Implement
**Priority:** P0 (Critical for production completeness)

> **⚠️ AI Assistants:** This file tracks the implementation of path resolution for ALL 15 supported languages. Update task statuses as you work. This is a major feature that brings Reflex to parity with other code intelligence tools.

---

## 🔄 ARCHITECTURAL DECISION #3 (2025-11-12)

**Decision:** Implement **deterministic path resolution** for all 15 languages with dependency extraction.

**Problem:**
- PHP is the ONLY language with complete path resolution (93.36% resolution rate in monorepos)
- All other 14 languages extract imports and classify them (Internal/External/Stdlib) but have `NULL` for `resolved_file_id`
- This means: Users can see what a file imports, but cannot navigate to the imported file
- Impact: 0% resolution rate for 14 out of 15 languages

**Solution:**
1. **Languages with config files (6 total):** Implement BOTH monorepo discovery AND path resolution
   - Go, Python, Java, Kotlin, Ruby (+ PHP already done)
   - Follow PHP's successful pattern: recursive config discovery + project_root tracking
2. **Languages without config files (9 total):** Implement path resolution only
   - Rust, TypeScript/JavaScript, Vue, Svelte, C, C++, C#, Zig
   - Use language-specific resolution rules (e.g., Rust module system, TS relative imports)

**Expected Outcome:**
- All 15 languages achieve 70-90%+ resolution rates for internal imports
- Consistent navigation experience across all languages
- Enables "Go to Definition" functionality for AI tools using Reflex

---

## 📊 Current State

### Languages with Path Resolution
1. **PHP** ✅ - 93.36% resolution rate (COMPLETE)
   - Monorepo support: ✅ (recursive composer.json discovery)
   - Path resolver: ✅ (`resolve_php_namespace_to_path()`)

### Languages WITHOUT Path Resolution (14 languages)
**Category 1: Has config files - Needs monorepo + resolver (5 languages)**
2. **Go** ❌ - 0% resolution
3. **Python** ❌ - 0% resolution
4. **Java** ❌ - 0% resolution
5. **Kotlin** ❌ - 0% resolution (reuses Java)
6. **Ruby** ⚠️ - 0% resolution (has partial monorepo support but no resolver)

**Category 2: No config files - Needs resolver only (9 languages)**
7. **Rust** ❌ - 0% resolution
8. **TypeScript/JavaScript** ❌ - 0% resolution
9. **Vue** ❌ - 0% resolution (reuses TS/JS logic)
10. **Svelte** ❌ - 0% resolution (reuses TS/JS logic)
11. **C** ❌ - 0% resolution
12. **C++** ❌ - 0% resolution
13. **C#** ❌ - 0% resolution
14. **Zig** ❌ - 0% resolution

---

## 🎯 Phase 1: High-Impact Languages (Python, Go, TS/JS, Rust)

### 1. Python Resolution (Estimated: 2-3 days)

#### 1.1 Monorepo Config Discovery
- [ ] **Add `find_all_python_configs()` function** (src/parsers/python.rs)
  - Recursively discover pyproject.toml, setup.py, setup.cfg files
  - Use `ignore::WalkBuilder` with gitignore support
  - Exclude vendor directories (site-packages, venv, .venv)
  - Return `Vec<PathBuf>` of discovered config files

- [ ] **Add `parse_all_python_packages()` function** (src/parsers/python.rs)
  - Call `find_all_python_configs()`
  - For each config: extract package_name + project_root (relative to index root)
  - Return `Vec<PythonPackageConfig>` with: package_name, project_root, config_path
  - Status: **PENDING**

#### 1.2 Path Resolution
- [ ] **Implement `resolve_python_import_to_path()` function** (src/parsers/python.rs)
  - Handle absolute imports: `django.conf.settings` → `django/conf/settings.py` or `__init__.py`
  - Handle relative imports: `.models` → `./models.py`, `..utils` → `../utils.py`
  - Check both `.py` files and `__init__.py` in directories
  - Prepend project_root from matching package config
  - Return `Option<String>` with resolved absolute path
  - Status: **PENDING**

#### 1.3 Indexer Integration
- [ ] **Update indexer.rs** (lines 591-594, 659-665)
  - Replace `find_python_package_name(root)` with `parse_all_python_packages(root)`
  - Store `Vec<PythonPackageConfig>` instead of single `Option<String>`
  - Pass configs to resolver in dependency resolution loop
  - Status: **PENDING**

- [ ] **Add Python resolver to indexer** (lines 688-715)
  - Add `else if file_path.ends_with(".py")` branch
  - Call `resolve_python_import_to_path()` with package configs
  - Look up resolved path in `dep_index.get_file_id_by_path()`
  - Set `resolved_file_id` if found
  - Status: **PENDING**

#### 1.4 Testing
- [ ] **Unit tests for Python resolver**
  - Test: Absolute import resolution (`mypackage.models.user`)
  - Test: Relative import resolution (`.models`, `..utils`)
  - Test: `__init__.py` package detection
  - Test: Monorepo with multiple Python packages
  - Status: **PENDING**

---

### 2. Go Resolution (Estimated: 2-3 days)

#### 2.1 Monorepo Config Discovery
- [ ] **Add `find_all_go_modules()` function** (src/parsers/go.rs)
  - Recursively discover go.mod files
  - Use `ignore::WalkBuilder` with gitignore support
  - Exclude vendor directories
  - Return `Vec<PathBuf>` of discovered go.mod files
  - Status: **PENDING**

- [ ] **Add `parse_all_go_modules()` function** (src/parsers/go.rs)
  - Call `find_all_go_modules()`
  - For each go.mod: extract module_name + project_root
  - Return `Vec<GoModuleConfig>` with: module_name, project_root, go_mod_path
  - Status: **PENDING**

#### 2.2 Path Resolution
- [ ] **Implement `resolve_go_import_to_path()` function** (src/parsers/go.rs)
  - Handle internal imports: `k8s.io/kubernetes/pkg/api` → `pkg/api/*.go`
  - Match import path against module prefixes
  - Strip module prefix and construct file path
  - Check for .go files in resolved directory
  - Prepend project_root from matching module config
  - Return `Option<String>` with resolved absolute path
  - Status: **PENDING**

#### 2.3 Indexer Integration
- [ ] **Update indexer.rs** (lines 579-582, 643-649)
  - Replace `find_go_module_name(root)` with `parse_all_go_modules(root)`
  - Store `Vec<GoModuleConfig>` instead of single `Option<String>`
  - Pass configs to resolver in dependency resolution loop
  - Status: **PENDING**

- [ ] **Add Go resolver to indexer** (lines 688-715)
  - Add `else if file_path.ends_with(".go")` branch
  - Call `resolve_go_import_to_path()` with module configs
  - Look up resolved path in database
  - Set `resolved_file_id` if found
  - Status: **PENDING**

#### 2.4 Testing
- [ ] **Unit tests for Go resolver**
  - Test: Internal import resolution with module prefix
  - Test: Multi-module monorepo (Kubernetes-style)
  - Test: Subdirectory imports
  - Test: No matching module (should return None)
  - Status: **PENDING**

---

### 3. TypeScript/JavaScript Resolution (Estimated: 2-3 days)

#### 3.1 Path Resolution (No monorepo config needed)
- [ ] **Implement `resolve_ts_import_to_path()` function** (src/parsers/typescript.rs)
  - Handle relative imports: `./Button` → `./Button.tsx` or `./Button/index.tsx`
  - Handle parent imports: `../utils/helper` → `../utils/helper.ts`
  - Try extensions in order: `.tsx`, `.ts`, `.jsx`, `.js`, `.mjs`, `.cjs`
  - Try index files: `./Button/index.tsx`, `./Button/index.ts`
  - Resolve path relative to current file's directory
  - Return `Option<String>` with resolved absolute path
  - Status: **PENDING**

#### 3.2 Indexer Integration
- [ ] **Add TS/JS resolver to indexer** (lines 688-715)
  - Add `else if file_path.ends_with(".ts") || file_path.ends_with(".tsx") || ...` branch
  - Call `resolve_ts_import_to_path()` with current_file path
  - Look up resolved path in database
  - Set `resolved_file_id` if found
  - Status: **PENDING**

#### 3.3 Testing
- [ ] **Unit tests for TS/JS resolver**
  - Test: Relative import `./Component`
  - Test: Parent import `../utils`
  - Test: Index file resolution `./Button` → `./Button/index.tsx`
  - Test: Multiple extension attempts (.tsx, .ts, .jsx, .js)
  - Test: Non-existent file (should return None)
  - Status: **PENDING**

---

### 4. Rust Resolution (Estimated: 2-3 days)

#### 4.1 Path Resolution (No monorepo config needed - Cargo handles workspaces)
- [ ] **Implement `resolve_rust_use_to_path()` function** (src/parsers/rust.rs)
  - Handle `crate::` imports: `crate::models::User` → `src/models/user.rs` or `src/models.rs`
  - Handle `super::` imports: resolve relative to parent module
  - Handle `self::` imports: resolve relative to current module
  - Apply Rust module system rules (mod.rs, file.rs naming)
  - Return `Option<String>` with resolved absolute path
  - Status: **PENDING**

#### 4.2 Indexer Integration
- [ ] **Add Rust resolver to indexer** (lines 688-715)
  - Add `else if file_path.ends_with(".rs")` branch
  - Call `resolve_rust_use_to_path()` with current_file path
  - Look up resolved path in database
  - Set `resolved_file_id` if found
  - Status: **PENDING**

#### 4.3 Testing
- [ ] **Unit tests for Rust resolver**
  - Test: `crate::models::User` resolution
  - Test: `super::utils` resolution
  - Test: `self::helper` resolution
  - Test: mod.rs vs file.rs naming conventions
  - Test: Nested module resolution
  - Status: **PENDING**

---

## 🎯 Phase 2: Enterprise Languages (Java, Kotlin, Ruby)

### 5. Java/Kotlin Resolution (Estimated: 3-4 days)

#### 5.1 Monorepo Config Discovery
- [ ] **Add `find_all_maven_gradle_projects()` function** (src/parsers/java.rs)
  - Recursively discover pom.xml and build.gradle files
  - Exclude target/ and build/ directories
  - Return `Vec<PathBuf>` of discovered project files
  - Status: **PENDING**

- [ ] **Add `parse_all_java_projects()` function** (src/parsers/java.rs)
  - Parse each pom.xml/build.gradle for groupId/package
  - Extract project_root for each
  - Return `Vec<JavaProjectConfig>` with: package_prefix, project_root, config_path
  - Status: **PENDING**

#### 5.2 Path Resolution
- [ ] **Implement `resolve_java_import_to_path()` function** (src/parsers/java.rs)
  - Convert package to path: `com.example.User` → `com/example/User.java`
  - Match against project package prefixes
  - Prepend project_root from matching config
  - Return `Option<String>` with resolved absolute path
  - Status: **PENDING**

- [ ] **Implement `resolve_kotlin_import_to_path()` function** (src/parsers/kotlin.rs)
  - Reuse Java logic but check for `.kt` extension
  - Handle Kotlin-specific imports
  - Status: **PENDING**

#### 5.3 Indexer Integration
- [ ] **Update indexer for Java/Kotlin** (lines 585-588, 603-607, 651-657, 675-681)
  - Replace single package discovery with `parse_all_java_projects()`
  - Add Java resolver branch in dependency resolution loop
  - Add Kotlin resolver branch (reuses Java configs)
  - Status: **PENDING**

#### 5.4 Testing
- [ ] **Unit tests for Java/Kotlin resolvers**
  - Test: Standard package resolution
  - Test: Multi-module Maven/Gradle monorepo
  - Test: Cross-project imports in monorepo
  - Status: **PENDING**

---

### 6. Ruby Resolution (Estimated: 2-3 days)

#### 6.1 Enhanced Monorepo Discovery
- [ ] **Update `find_ruby_gem_names()` function** (src/parsers/ruby.rs)
  - Remove `.max_depth(3)` limit
  - Add project_root tracking for each .gemspec
  - Return `Vec<RubyGemConfig>` instead of `Vec<String>`
  - Status: **PENDING**

#### 6.2 Path Resolution
- [ ] **Implement `resolve_ruby_require_to_path()` function** (src/parsers/ruby.rs)
  - Handle gem-relative requires: `my_gem/models/user` → `lib/models/user.rb`
  - Handle `require_relative`: resolve relative to current file
  - Match against gem name variants (underscores vs hyphens)
  - Prepend project_root from matching gem config
  - Return `Option<String>` with resolved absolute path
  - Status: **PENDING**

#### 6.3 Indexer Integration
- [ ] **Update indexer for Ruby** (lines 597-600, 667-673)
  - Replace `find_ruby_gem_names()` with new version returning configs
  - Add Ruby resolver branch in dependency resolution loop
  - Status: **PENDING**

#### 6.4 Testing
- [ ] **Unit tests for Ruby resolver**
  - Test: Gem-relative require resolution
  - Test: `require_relative` resolution
  - Test: Gem name variant matching (my_gem vs my-gem)
  - Test: Multi-gem monorepo
  - Status: **PENDING**

---

## 🎯 Phase 3: Systems Languages (C, C++, C#, Zig, Vue, Svelte)

### 7. C/C++ Resolution (Estimated: 2-3 days)

#### 7.1 Path Resolution
- [ ] **Implement `resolve_c_include_to_path()` function** (src/parsers/c.rs)
  - Handle `#include "local.h"`: search in same directory first
  - Handle `#include <system.h>`: skip (stdlib)
  - Check common include paths (./include, ../include)
  - Return `Option<String>` with resolved absolute path
  - Status: **PENDING**

- [ ] **Implement `resolve_cpp_include_to_path()` function** (src/parsers/cpp.rs)
  - Reuse C logic with C++-specific extensions (.hpp, .hxx, .H)
  - Status: **PENDING**

#### 7.2 Indexer Integration
- [ ] **Add C/C++ resolvers to indexer** (lines 688-715)
  - Add branches for `.c`, `.h`, `.cpp`, `.hpp`, etc.
  - Call respective resolvers
  - Status: **PENDING**

#### 7.3 Testing
- [ ] **Unit tests for C/C++ resolvers**
  - Test: Local include resolution `#include "myheader.h"`
  - Test: Parent directory includes
  - Test: System includes are skipped
  - Status: **PENDING**

---

### 8. C# Resolution (Estimated: 2 days)

#### 8.1 Path Resolution
- [ ] **Implement `resolve_csharp_using_to_path()` function** (src/parsers/csharp.rs)
  - Convert namespace to path: `MyNamespace.Models.User` → `MyNamespace/Models/User.cs`
  - Check for file existence
  - Return `Option<String>` with resolved absolute path
  - Status: **PENDING**

#### 8.2 Indexer Integration & Testing
- [ ] **Add C# resolver to indexer and write tests**
  - Status: **PENDING**

---

### 9. Zig Resolution (Estimated: 1-2 days)

#### 9.1 Path Resolution
- [ ] **Implement `resolve_zig_import_to_path()` function** (src/parsers/zig.rs)
  - Handle `@import("...")` paths
  - Resolve relative to current file
  - Return `Option<String>` with resolved absolute path
  - Status: **PENDING**

#### 9.2 Indexer Integration & Testing
- [ ] **Add Zig resolver to indexer and write tests**
  - Status: **PENDING**

---

### 10. Vue/Svelte Resolution (Estimated: 1-2 days)

#### 10.1 Path Resolution
- [ ] **Implement Vue/Svelte resolvers** (src/parsers/vue.rs, src/parsers/svelte.rs)
  - Reuse TypeScript/JavaScript resolver logic
  - Check for `.vue` and `.svelte` extensions
  - Status: **PENDING**

#### 10.2 Indexer Integration & Testing
- [ ] **Add Vue/Svelte resolvers to indexer and write tests**
  - Status: **PENDING**

---

## 🧪 Integration & Testing

### Integration Tasks
- [ ] **Test all resolvers with real-world monorepos**
  - Test inteleworx-platform (user's PHP/Vue monorepo)
  - Test Kubernetes (Go multi-module monorepo)
  - Test Django (Python multi-package monorepo)
  - Status: **PENDING**

- [ ] **Benchmark query performance overhead**
  - Measure query time with vs without path resolution
  - Target: <5% overhead per query
  - Status: **PENDING**

### Documentation Tasks
- [ ] **Update CLAUDE.md with resolver implementation details**
  - Document path resolution architecture
  - Add examples for each language
  - Status: **PENDING**

---

## 📈 Success Metrics

**Target Resolution Rates:**
- Languages with monorepo support: 85-95% (PHP, Go, Python, Java, Kotlin, Ruby)
- Languages without config files: 70-85% (Rust, TS/JS, Vue, Svelte, C, C++, C#, Zig)

**Performance Targets:**
- Path resolution overhead: <5% per query
- Monorepo config discovery: <1s during indexing
- No impact on incremental indexing speed

---

## 📝 Implementation Notes

### Pattern to Follow (from PHP's success)

**For languages with config files:**
1. Create `find_all_<config_files>()` - recursive discovery
2. Create `parse_all_<language>_configs()` - aggregate all configs with project_root
3. Create `resolve_<language>_import_to_path()` - match import to project, apply resolution rules
4. Update indexer to use new functions and call resolver

**For languages without config files:**
1. Create `resolve_<language>_import_to_path()` - use language-specific resolution rules
2. Update indexer to call resolver

**General principles:**
- Use `ignore::WalkBuilder` for gitignore-aware discovery
- Exclude vendor/build directories (vendor/, node_modules/, target/, build/)
- Track project_root as relative path from index root
- Return `Option<String>` from resolvers (None if can't resolve)
- Log trace messages for debugging resolution failures

---

**END OF DEPS_TODO.md**
