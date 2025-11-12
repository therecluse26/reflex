//! Dependency tracking and graph analysis
//!
//! This module provides functionality for tracking file dependencies (imports/includes)
//! and analyzing the dependency graph of a codebase.
//!
//! # Architecture
//!
//! The system uses a "depth-1 storage" approach:
//! - Only direct dependencies are stored in the database
//! - Deeper relationships are computed on-demand via graph traversal
//! - This provides O(n) storage while enabling any-depth queries
//!
//! # Example
//!
//! ```no_run
//! use reflex::dependency::DependencyIndex;
//! use reflex::cache::CacheManager;
//!
//! let cache = CacheManager::new(".");
//! let deps = DependencyIndex::new(cache);
//!
//! // Get direct dependencies of a file
//! let file_deps = deps.get_dependencies(42)?;
//!
//! // Get files that import this file (reverse lookup)
//! let dependents = deps.get_dependents(42)?;
//!
//! // Traverse dependency graph to depth 3
//! let transitive = deps.get_transitive_deps(42, 3)?;
//! # Ok::<(), anyhow::Error>(())
//! ```

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::cache::CacheManager;
use crate::models::{Dependency, DependencyInfo, ImportType};

/// Manages dependency storage and graph operations
pub struct DependencyIndex {
    cache: CacheManager,
}

impl DependencyIndex {
    /// Create a new dependency index for the given cache
    pub fn new(cache: CacheManager) -> Self {
        Self { cache }
    }

    /// Insert a dependency into the database
    ///
    /// # Arguments
    ///
    /// * `file_id` - Source file ID
    /// * `imported_path` - Import path as written in source
    /// * `resolved_file_id` - Resolved target file ID (None if external/stdlib)
    /// * `import_type` - Type of import (internal/external/stdlib)
    /// * `line_number` - Line where import appears
    /// * `imported_symbols` - Optional list of imported symbols
    pub fn insert_dependency(
        &self,
        file_id: i64,
        imported_path: String,
        resolved_file_id: Option<i64>,
        import_type: ImportType,
        line_number: usize,
        imported_symbols: Option<Vec<String>>,
    ) -> Result<()> {
        let db_path = self.cache.path().join("meta.db");
        let conn = Connection::open(&db_path)
            .context("Failed to open meta.db for dependency insert")?;

        let import_type_str = match import_type {
            ImportType::Internal => "internal",
            ImportType::External => "external",
            ImportType::Stdlib => "stdlib",
        };

        let symbols_json = imported_symbols
            .as_ref()
            .map(|syms| serde_json::to_string(syms).unwrap_or_else(|_| "[]".to_string()));

        conn.execute(
            "INSERT INTO file_dependencies (file_id, imported_path, resolved_file_id, import_type, line_number, imported_symbols)
             VALUES (?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                file_id,
                imported_path,
                resolved_file_id,
                import_type_str,
                line_number as i64,
                symbols_json,
            ],
        )?;

        Ok(())
    }

    /// Batch insert multiple dependencies in a single transaction
    ///
    /// More efficient than individual inserts for bulk operations.
    pub fn batch_insert_dependencies(&self, dependencies: &[Dependency]) -> Result<()> {
        if dependencies.is_empty() {
            return Ok(());
        }

        let db_path = self.cache.path().join("meta.db");
        let mut conn = Connection::open(&db_path)
            .context("Failed to open meta.db for batch dependency insert")?;

        let tx = conn.transaction()?;

        for dep in dependencies {
            let import_type_str = match dep.import_type {
                ImportType::Internal => "internal",
                ImportType::External => "external",
                ImportType::Stdlib => "stdlib",
            };

            let symbols_json = dep
                .imported_symbols
                .as_ref()
                .map(|syms| serde_json::to_string(syms).unwrap_or_else(|_| "[]".to_string()));

            tx.execute(
                "INSERT INTO file_dependencies (file_id, imported_path, resolved_file_id, import_type, line_number, imported_symbols)
                 VALUES (?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    dep.file_id,
                    dep.imported_path,
                    dep.resolved_file_id,
                    import_type_str,
                    dep.line_number as i64,
                    symbols_json,
                ],
            )?;
        }

        tx.commit()?;
        log::debug!("Batch inserted {} dependencies", dependencies.len());
        Ok(())
    }

    /// Get all direct dependencies for a file
    ///
    /// Returns a list of files/modules that this file imports.
    pub fn get_dependencies(&self, file_id: i64) -> Result<Vec<Dependency>> {
        let db_path = self.cache.path().join("meta.db");
        let conn = Connection::open(&db_path)
            .context("Failed to open meta.db for dependency lookup")?;

        let mut stmt = conn.prepare(
            "SELECT file_id, imported_path, resolved_file_id, import_type, line_number, imported_symbols
             FROM file_dependencies
             WHERE file_id = ?
             ORDER BY line_number",
        )?;

        let deps = stmt
            .query_map([file_id], |row| {
                let import_type_str: String = row.get(3)?;
                let import_type = match import_type_str.as_str() {
                    "internal" => ImportType::Internal,
                    "external" => ImportType::External,
                    "stdlib" => ImportType::Stdlib,
                    _ => ImportType::External,
                };

                let symbols_json: Option<String> = row.get(5)?;
                let imported_symbols = symbols_json.and_then(|json| {
                    serde_json::from_str(&json).ok()
                });

                Ok(Dependency {
                    file_id: row.get(0)?,
                    imported_path: row.get(1)?,
                    resolved_file_id: row.get(2)?,
                    import_type,
                    line_number: row.get::<_, i64>(4)? as usize,
                    imported_symbols,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(deps)
    }

    /// Get all files that depend on this file (reverse lookup)
    ///
    /// Returns a list of file IDs that import this file.
    pub fn get_dependents(&self, file_id: i64) -> Result<Vec<i64>> {
        let db_path = self.cache.path().join("meta.db");
        let conn = Connection::open(&db_path)
            .context("Failed to open meta.db for reverse dependency lookup")?;

        let mut stmt = conn.prepare(
            "SELECT DISTINCT file_id
             FROM file_dependencies
             WHERE resolved_file_id = ?
             ORDER BY file_id",
        )?;

        let dependents = stmt
            .query_map([file_id], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(dependents)
    }

    /// Get dependencies as DependencyInfo (for API output)
    ///
    /// Converts internal Dependency records to simplified DependencyInfo
    /// suitable for JSON output.
    pub fn get_dependencies_info(&self, file_id: i64) -> Result<Vec<DependencyInfo>> {
        let deps = self.get_dependencies(file_id)?;

        let dep_infos = deps
            .into_iter()
            .map(|dep| {
                // Try to get the resolved path (all deps are internal now)
                let path = if let Some(resolved_id) = dep.resolved_file_id {
                    // Try to get the actual file path
                    self.get_file_path(resolved_id).unwrap_or(dep.imported_path)
                } else {
                    dep.imported_path
                };

                DependencyInfo {
                    path,
                    line: Some(dep.line_number),
                    symbols: dep.imported_symbols,
                }
            })
            .collect();

        Ok(dep_infos)
    }

    /// Get transitive dependencies up to a given depth
    ///
    /// Traverses the dependency graph using BFS to find all dependencies
    /// reachable within the specified depth.
    ///
    /// # Arguments
    ///
    /// * `file_id` - Starting file ID
    /// * `max_depth` - Maximum traversal depth (0 = only direct deps)
    ///
    /// # Returns
    ///
    /// HashMap mapping file_id to depth (distance from start file)
    pub fn get_transitive_deps(&self, file_id: i64, max_depth: usize) -> Result<HashMap<i64, usize>> {
        let mut visited = HashMap::new();
        let mut queue = VecDeque::new();

        // Start with the initial file at depth 0
        queue.push_back((file_id, 0));
        visited.insert(file_id, 0);

        while let Some((current_id, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            // Get direct dependencies (only internal ones with resolved IDs)
            let deps = self.get_dependencies(current_id)?;

            for dep in deps {
                if let Some(resolved_id) = dep.resolved_file_id {
                    // Only visit if we haven't seen it or found a shorter path
                    if !visited.contains_key(&resolved_id) {
                        visited.insert(resolved_id, depth + 1);
                        queue.push_back((resolved_id, depth + 1));
                    }
                }
            }
        }

        Ok(visited)
    }

    /// Detect circular dependencies in the entire codebase
    ///
    /// Uses depth-first search to find cycles in the dependency graph.
    ///
    /// Returns a list of cycle paths, where each cycle is represented as
    /// a vector of file IDs forming the cycle.
    pub fn detect_circular_dependencies(&self) -> Result<Vec<Vec<i64>>> {
        let all_files = self.get_all_file_ids()?;
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();
        let mut cycles = Vec::new();

        for file_id in all_files {
            if !visited.contains(&file_id) {
                self.dfs_cycle_detect(
                    file_id,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                    &mut cycles,
                )?;
            }
        }

        Ok(cycles)
    }

    /// DFS helper for cycle detection
    fn dfs_cycle_detect(
        &self,
        file_id: i64,
        visited: &mut HashSet<i64>,
        rec_stack: &mut HashSet<i64>,
        path: &mut Vec<i64>,
        cycles: &mut Vec<Vec<i64>>,
    ) -> Result<()> {
        visited.insert(file_id);
        rec_stack.insert(file_id);
        path.push(file_id);

        let deps = self.get_dependencies(file_id)?;

        for dep in deps {
            if let Some(resolved_id) = dep.resolved_file_id {
                if !visited.contains(&resolved_id) {
                    self.dfs_cycle_detect(resolved_id, visited, rec_stack, path, cycles)?;
                } else if rec_stack.contains(&resolved_id) {
                    // Found a cycle! Extract it from path
                    if let Some(cycle_start) = path.iter().position(|&id| id == resolved_id) {
                        let cycle = path[cycle_start..].to_vec();
                        cycles.push(cycle);
                    }
                }
            }
        }

        path.pop();
        rec_stack.remove(&file_id);

        Ok(())
    }

    /// Get file paths for a list of file IDs
    ///
    /// Useful for converting file ID results to human-readable paths.
    pub fn get_file_paths(&self, file_ids: &[i64]) -> Result<HashMap<i64, String>> {
        let db_path = self.cache.path().join("meta.db");
        let conn = Connection::open(&db_path)
            .context("Failed to open meta.db for file path lookup")?;

        let mut paths = HashMap::new();

        for &file_id in file_ids {
            if let Ok(path) = conn.query_row(
                "SELECT path FROM files WHERE id = ?",
                [file_id],
                |row| row.get::<_, String>(0),
            ) {
                paths.insert(file_id, path);
            }
        }

        Ok(paths)
    }

    /// Get file path for a single file ID
    fn get_file_path(&self, file_id: i64) -> Result<String> {
        let db_path = self.cache.path().join("meta.db");
        let conn = Connection::open(&db_path)
            .context("Failed to open meta.db for file path lookup")?;

        let path = conn.query_row(
            "SELECT path FROM files WHERE id = ?",
            [file_id],
            |row| row.get::<_, String>(0),
        )?;

        Ok(path)
    }

    /// Get all file IDs in the database
    fn get_all_file_ids(&self) -> Result<Vec<i64>> {
        let db_path = self.cache.path().join("meta.db");
        let conn = Connection::open(&db_path)
            .context("Failed to open meta.db for file ID lookup")?;

        let mut stmt = conn.prepare("SELECT id FROM files")?;
        let file_ids = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(file_ids)
    }

    /// Find hotspots (most imported files)
    ///
    /// Returns a list of (file_id, count) tuples sorted by import count descending.
    pub fn find_hotspots(&self, limit: Option<usize>) -> Result<Vec<(i64, usize)>> {
        let db_path = self.cache.path().join("meta.db");
        let conn = Connection::open(&db_path)
            .context("Failed to open meta.db for hotspot analysis")?;

        let query = if let Some(limit) = limit {
            format!(
                "SELECT resolved_file_id, COUNT(*) as count
                 FROM file_dependencies
                 WHERE resolved_file_id IS NOT NULL
                 GROUP BY resolved_file_id
                 ORDER BY count DESC
                 LIMIT {}",
                limit
            )
        } else {
            "SELECT resolved_file_id, COUNT(*) as count
             FROM file_dependencies
             WHERE resolved_file_id IS NOT NULL
             GROUP BY resolved_file_id
             ORDER BY count DESC"
                .to_string()
        };

        let mut stmt = conn.prepare(&query)?;
        let hotspots = stmt
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)? as usize))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(hotspots)
    }

    /// Find unused files (files with no incoming dependencies)
    ///
    /// These are potential candidates for deletion.
    pub fn find_unused_files(&self) -> Result<Vec<i64>> {
        let db_path = self.cache.path().join("meta.db");
        let conn = Connection::open(&db_path)
            .context("Failed to open meta.db for unused file analysis")?;

        let mut stmt = conn.prepare(
            "SELECT id FROM files
             WHERE id NOT IN (
                 SELECT DISTINCT resolved_file_id
                 FROM file_dependencies
                 WHERE resolved_file_id IS NOT NULL
             )",
        )?;

        let unused = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(unused)
    }

    /// Clear all dependencies for a file (used during incremental reindexing)
    pub fn clear_dependencies(&self, file_id: i64) -> Result<()> {
        let db_path = self.cache.path().join("meta.db");
        let conn = Connection::open(&db_path)
            .context("Failed to open meta.db for dependency clearing")?;

        conn.execute(
            "DELETE FROM file_dependencies WHERE file_id = ?",
            [file_id],
        )?;

        Ok(())
    }

    /// Get file ID by path
    ///
    /// Returns None if the file is not in the index.
    pub fn get_file_id_by_path(&self, path: &str) -> Result<Option<i64>> {
        let db_path = self.cache.path().join("meta.db");
        let conn = Connection::open(&db_path)
            .context("Failed to open meta.db for file ID lookup")?;

        match conn.query_row(
            "SELECT id FROM files WHERE path = ?",
            [path],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(id) => Ok(Some(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

/// Resolve a Rust import path to an absolute file path
///
/// This function handles Rust-specific path resolution rules:
/// - `crate::` - Starts from crate root (src/lib.rs or src/main.rs)
/// - `super::` - Goes up one module level
/// - `self::` - Stays in current module
/// - `mod name` - Looks for name.rs or name/mod.rs
/// - External crates - Returns None
///
/// # Arguments
///
/// * `import_path` - The import path as written in source (e.g., "crate::models::Language")
/// * `current_file` - Path to the file containing the import (e.g., "src/query.rs")
/// * `project_root` - Root directory of the project
///
/// # Returns
///
/// `Some(path)` if the import resolves to a project file, `None` if it's external/stdlib
pub fn resolve_rust_import(
    import_path: &str,
    current_file: &str,
    project_root: &std::path::Path,
) -> Option<String> {
    use std::path::{Path, PathBuf};

    // External crates and stdlib - don't resolve
    if !import_path.starts_with("crate::")
        && !import_path.starts_with("super::")
        && !import_path.starts_with("self::")
    {
        return None;
    }

    let current_path = Path::new(current_file);
    let mut resolved_path: Option<PathBuf> = None;

    if import_path.starts_with("crate::") {
        // Start from crate root (src/lib.rs or src/main.rs)
        let crate_root = if project_root.join("src/lib.rs").exists() {
            project_root.join("src")
        } else if project_root.join("src/main.rs").exists() {
            project_root.join("src")
        } else {
            // Fallback to src/ directory
            project_root.join("src")
        };

        let path_parts: Vec<&str> = import_path
            .strip_prefix("crate::")
            .unwrap()
            .split("::")
            .collect();

        resolved_path = resolve_module_path(&crate_root, &path_parts);
    } else if import_path.starts_with("super::") {
        // Go up one directory from current file's parent (the current module's parent)
        if let Some(current_dir) = current_path.parent() {
            if let Some(parent_dir) = current_dir.parent() {
                let path_parts: Vec<&str> = import_path
                    .strip_prefix("super::")
                    .unwrap()
                    .split("::")
                    .collect();

                resolved_path = resolve_module_path(parent_dir, &path_parts);
            }
        }
    } else if import_path.starts_with("self::") {
        // Stay in current directory
        if let Some(current_dir) = current_path.parent() {
            let path_parts: Vec<&str> = import_path
                .strip_prefix("self::")
                .unwrap()
                .split("::")
                .collect();

            resolved_path = resolve_module_path(current_dir, &path_parts);
        }
    }

    // Convert to string and make relative to project root
    resolved_path.and_then(|p| {
        p.strip_prefix(project_root)
            .ok()
            .map(|rel| rel.to_string_lossy().to_string())
    })
}

/// Resolve a module path given a starting directory and path components
///
/// Handles Rust's module system rules:
/// - `foo` → check foo.rs or foo/mod.rs
/// - `foo::bar` → check foo/bar.rs or foo/bar/mod.rs
fn resolve_module_path(start_dir: &std::path::Path, components: &[&str]) -> Option<std::path::PathBuf> {

    if components.is_empty() {
        return None;
    }

    let mut current = start_dir.to_path_buf();

    // For all components except the last, they must be directories
    for &component in &components[..components.len() - 1] {
        // Try as a directory with mod.rs
        let dir_path = current.join(component);
        let mod_file = dir_path.join("mod.rs");

        if mod_file.exists() {
            current = dir_path;
        } else {
            // Component must be a directory for nested paths
            return None;
        }
    }

    // For the last component, try both file.rs and file/mod.rs
    let last_component = components.last().unwrap();

    // Try as a single file
    let file_path = current.join(format!("{}.rs", last_component));
    if file_path.exists() {
        return Some(file_path);
    }

    // Try as a directory with mod.rs
    let dir_path = current.join(last_component);
    let mod_file = dir_path.join("mod.rs");
    if mod_file.exists() {
        return Some(mod_file);
    }

    None
}

/// Resolve a `mod` declaration to a file path
///
/// For `mod parser;`, this checks for:
/// - `parser.rs` (sibling file)
/// - `parser/mod.rs` (directory module)
pub fn resolve_rust_mod_declaration(
    mod_name: &str,
    current_file: &str,
    _project_root: &std::path::Path,
) -> Option<String> {
    use std::path::Path;

    let current_path = Path::new(current_file);
    let current_dir = current_path.parent()?;

    // Try sibling file
    let sibling = current_dir.join(format!("{}.rs", mod_name));
    if sibling.exists() {
        return Some(sibling.to_string_lossy().to_string());
    }

    // Try directory module
    let dir_mod = current_dir.join(mod_name).join("mod.rs");
    if dir_mod.exists() {
        return Some(dir_mod.to_string_lossy().to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_cache() -> (TempDir, CacheManager) {
        let temp = TempDir::new().unwrap();
        let cache = CacheManager::new(temp.path());
        cache.init().unwrap();

        // Add some test files
        cache.update_file("src/main.rs", "rust", 100).unwrap();
        cache.update_file("src/lib.rs", "rust", 50).unwrap();
        cache.update_file("src/utils.rs", "rust", 30).unwrap();

        (temp, cache)
    }

    #[test]
    fn test_insert_and_get_dependencies() {
        let (_temp, cache) = setup_test_cache();
        let deps_index = DependencyIndex::new(cache);

        // Get file IDs
        let main_id = 1i64;
        let lib_id = 2i64;

        // Insert a dependency: main.rs imports lib.rs
        deps_index
            .insert_dependency(
                main_id,
                "crate::lib".to_string(),
                Some(lib_id),
                ImportType::Internal,
                5,
                None,
            )
            .unwrap();

        // Retrieve dependencies
        let deps = deps_index.get_dependencies(main_id).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].imported_path, "crate::lib");
        assert_eq!(deps[0].resolved_file_id, Some(lib_id));
        assert_eq!(deps[0].import_type, ImportType::Internal);
    }

    #[test]
    fn test_reverse_lookup() {
        let (_temp, cache) = setup_test_cache();
        let deps_index = DependencyIndex::new(cache);

        let main_id = 1i64;
        let lib_id = 2i64;
        let utils_id = 3i64;

        // main.rs imports lib.rs
        deps_index
            .insert_dependency(
                main_id,
                "crate::lib".to_string(),
                Some(lib_id),
                ImportType::Internal,
                5,
                None,
            )
            .unwrap();

        // utils.rs also imports lib.rs
        deps_index
            .insert_dependency(
                utils_id,
                "crate::lib".to_string(),
                Some(lib_id),
                ImportType::Internal,
                3,
                None,
            )
            .unwrap();

        // Get files that import lib.rs
        let dependents = deps_index.get_dependents(lib_id).unwrap();
        assert_eq!(dependents.len(), 2);
        assert!(dependents.contains(&main_id));
        assert!(dependents.contains(&utils_id));
    }

    #[test]
    fn test_transitive_dependencies() {
        let (_temp, cache) = setup_test_cache();
        let deps_index = DependencyIndex::new(cache);

        let file1 = 1i64;
        let file2 = 2i64;
        let file3 = 3i64;

        // file1 → file2 → file3
        deps_index
            .insert_dependency(
                file1,
                "file2".to_string(),
                Some(file2),
                ImportType::Internal,
                1,
                None,
            )
            .unwrap();

        deps_index
            .insert_dependency(
                file2,
                "file3".to_string(),
                Some(file3),
                ImportType::Internal,
                1,
                None,
            )
            .unwrap();

        // Get transitive deps at depth 2
        let transitive = deps_index.get_transitive_deps(file1, 2).unwrap();

        // Should include file1 (depth 0), file2 (depth 1), file3 (depth 2)
        assert_eq!(transitive.len(), 3);
        assert_eq!(transitive.get(&file1), Some(&0));
        assert_eq!(transitive.get(&file2), Some(&1));
        assert_eq!(transitive.get(&file3), Some(&2));
    }

    #[test]
    fn test_batch_insert() {
        let (_temp, cache) = setup_test_cache();
        let deps_index = DependencyIndex::new(cache);

        let deps = vec![
            Dependency {
                file_id: 1,
                imported_path: "std::collections".to_string(),
                resolved_file_id: None,
                import_type: ImportType::Stdlib,
                line_number: 1,
                imported_symbols: Some(vec!["HashMap".to_string()]),
            },
            Dependency {
                file_id: 1,
                imported_path: "crate::lib".to_string(),
                resolved_file_id: Some(2),
                import_type: ImportType::Internal,
                line_number: 2,
                imported_symbols: None,
            },
        ];

        deps_index.batch_insert_dependencies(&deps).unwrap();

        let retrieved = deps_index.get_dependencies(1).unwrap();
        assert_eq!(retrieved.len(), 2);
    }

    #[test]
    fn test_clear_dependencies() {
        let (_temp, cache) = setup_test_cache();
        let deps_index = DependencyIndex::new(cache);

        // Insert dependencies
        deps_index
            .insert_dependency(
                1,
                "crate::lib".to_string(),
                Some(2),
                ImportType::Internal,
                1,
                None,
            )
            .unwrap();

        // Verify they exist
        assert_eq!(deps_index.get_dependencies(1).unwrap().len(), 1);

        // Clear them
        deps_index.clear_dependencies(1).unwrap();

        // Verify they're gone
        assert_eq!(deps_index.get_dependencies(1).unwrap().len(), 0);
    }

    #[test]
    fn test_resolve_rust_import_crate() {
        use std::fs;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let project_root = temp.path();

        // Create directory structure
        fs::create_dir_all(project_root.join("src")).unwrap();
        fs::write(project_root.join("src/lib.rs"), "").unwrap();
        fs::write(project_root.join("src/models.rs"), "").unwrap();

        // Test crate:: resolution
        let resolved = resolve_rust_import(
            "crate::models",
            "src/query.rs",
            project_root,
        );

        assert_eq!(resolved, Some("src/models.rs".to_string()));
    }

    #[test]
    fn test_resolve_rust_import_super() {
        use std::fs;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let project_root = temp.path();

        // Create directory structure: src/parsers/rust.rs needs to import src/models.rs
        fs::create_dir_all(project_root.join("src/parsers")).unwrap();
        fs::write(project_root.join("src/models.rs"), "").unwrap();
        fs::write(project_root.join("src/parsers/rust.rs"), "").unwrap();

        // Test super:: resolution from parsers/rust.rs
        // Use absolute path for current_file
        let current_file = project_root.join("src/parsers/rust.rs");
        let resolved = resolve_rust_import(
            "super::models",
            &current_file.to_string_lossy(),
            project_root,
        );

        assert_eq!(resolved, Some("src/models.rs".to_string()));
    }

    #[test]
    fn test_resolve_rust_import_external() {
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let project_root = temp.path();

        // External crates should return None
        let resolved = resolve_rust_import(
            "serde::Serialize",
            "src/models.rs",
            project_root,
        );

        assert_eq!(resolved, None);

        // Stdlib should return None
        let resolved = resolve_rust_import(
            "std::collections::HashMap",
            "src/models.rs",
            project_root,
        );

        assert_eq!(resolved, None);
    }

    #[test]
    fn test_resolve_rust_mod_declaration() {
        use std::fs;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let project_root = temp.path();

        // Create directory structure
        fs::create_dir_all(project_root.join("src")).unwrap();
        fs::write(project_root.join("src/lib.rs"), "").unwrap();
        fs::write(project_root.join("src/parser.rs"), "").unwrap();

        // Test mod declaration resolution
        let resolved = resolve_rust_mod_declaration(
            "parser",
            &project_root.join("src/lib.rs").to_string_lossy(),
            project_root,
        );

        assert!(resolved.is_some());
        assert!(resolved.unwrap().ends_with("src/parser.rs"));
    }

    #[test]
    fn test_resolve_rust_import_nested() {
        use std::fs;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let project_root = temp.path();

        // Create directory structure: src/models/language.rs
        fs::create_dir_all(project_root.join("src/models")).unwrap();
        fs::write(project_root.join("src/models/mod.rs"), "").unwrap();
        fs::write(project_root.join("src/models/language.rs"), "").unwrap();

        // Test nested module resolution
        let resolved = resolve_rust_import(
            "crate::models::language",
            "src/query.rs",
            project_root,
        );

        assert_eq!(resolved, Some("src/models/language.rs".to_string()));
    }
}
