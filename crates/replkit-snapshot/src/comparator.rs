use std::fs;
use std::path::{Path, PathBuf};
use crate::capture::Snapshot;
use crate::error::{ComparisonError, Result};

#[derive(Debug, Clone)]
pub struct SnapshotComparator {
    snapshot_directory: PathBuf,
    update_mode: bool,
}

#[derive(Debug, Clone)]
pub struct ComparisonResult {
    pub snapshot_name: String,
    pub matches: bool,
    pub golden_file_path: PathBuf,
    pub diff: Option<String>,
    pub action_taken: ComparisonAction,
}

#[derive(Debug, Clone)]
pub enum ComparisonAction {
    Matched,
    Created,
    Updated,
    Failed(String),
}

impl SnapshotComparator {
    pub fn new(snapshot_directory: PathBuf, update_mode: bool) -> Result<Self> {
        // Create snapshot directory if it doesn't exist
        if !snapshot_directory.exists() {
            fs::create_dir_all(&snapshot_directory)
                .map_err(|e| ComparisonError::IoError(format!(
                    "Failed to create snapshot directory '{}': {}", 
                    snapshot_directory.display(), 
                    e
                )))?;
        }
        
        Ok(Self {
            snapshot_directory,
            update_mode,
        })
    }
    
    pub fn compare_snapshot(&self, snapshot: &Snapshot) -> Result<ComparisonResult> {
        let golden_file_path = self.get_golden_file_path(&snapshot.name);
        
        // Check if golden file exists
        if !golden_file_path.exists() {
            if self.update_mode {
                // Create new golden file
                self.save_snapshot_to_file(snapshot, &golden_file_path)?;
                Ok(ComparisonResult {
                    snapshot_name: snapshot.name.clone(),
                    matches: true,
                    golden_file_path,
                    diff: None,
                    action_taken: ComparisonAction::Created,
                })
            } else {
                // Golden file missing in non-update mode
                Err(ComparisonError::GoldenFileMissing(format!(
                    "Golden file '{}' does not exist. Run with --update to create it.",
                    golden_file_path.display()
                )).into())
            }
        } else {
            // Load existing golden file
            let golden_content = self.load_golden_file(&golden_file_path)?;
            
            if snapshot.content == golden_content {
                // Perfect match
                Ok(ComparisonResult {
                    snapshot_name: snapshot.name.clone(),
                    matches: true,
                    golden_file_path,
                    diff: None,
                    action_taken: ComparisonAction::Matched,
                })
            } else {
                // Content differs
                if self.update_mode {
                    // Update golden file with new content
                    self.save_snapshot_to_file(snapshot, &golden_file_path)?;
                    Ok(ComparisonResult {
                        snapshot_name: snapshot.name.clone(),
                        matches: true,
                        golden_file_path,
                        diff: Some(self.compute_diff(&golden_content, &snapshot.content)),
                        action_taken: ComparisonAction::Updated,
                    })
                } else {
                    // Return diff for manual inspection
                    let diff = self.compute_diff(&golden_content, &snapshot.content);
                    Ok(ComparisonResult {
                        snapshot_name: snapshot.name.clone(),
                        matches: false,
                        golden_file_path,
                        diff: Some(diff),
                        action_taken: ComparisonAction::Failed("Content mismatch".to_string()),
                    })
                }
            }
        }
    }
    
    pub fn compare_multiple_snapshots(&self, snapshots: &[Snapshot]) -> Result<Vec<ComparisonResult>> {
        let mut results = Vec::new();
        
        for snapshot in snapshots {
            match self.compare_snapshot(snapshot) {
                Ok(result) => results.push(result),
                Err(e) => {
                    results.push(ComparisonResult {
                        snapshot_name: snapshot.name.clone(),
                        matches: false,
                        golden_file_path: self.get_golden_file_path(&snapshot.name),
                        diff: None,
                        action_taken: ComparisonAction::Failed(e.to_string()),
                    });
                }
            }
        }
        
        Ok(results)
    }
    
    fn get_golden_file_path(&self, snapshot_name: &str) -> PathBuf {
        // Sanitize snapshot name for filesystem use
        let safe_name = self.sanitize_filename(snapshot_name);
        self.snapshot_directory.join(format!("{}.golden", safe_name))
    }
    
    fn sanitize_filename(&self, name: &str) -> String {
        // Replace invalid filesystem characters with underscores
        name.chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                c if c.is_control() => '_',
                c => c,
            })
            .collect()
    }
    
    fn save_snapshot_to_file(&self, snapshot: &Snapshot, file_path: &Path) -> Result<()> {
        // Create directory if it doesn't exist
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| ComparisonError::IoError(format!(
                    "Failed to create directory '{}': {}", 
                    parent.display(), 
                    e
                )))?;
        }
        
        // Save snapshot content with metadata header
        let file_content = self.create_golden_file_content(snapshot);
        fs::write(file_path, file_content)
            .map_err(|e| ComparisonError::IoError(format!(
                "Failed to write golden file '{}': {}", 
                file_path.display(), 
                e
            )))?;
        
        Ok(())
    }
    
    fn load_golden_file(&self, file_path: &Path) -> Result<String> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| ComparisonError::IoError(format!(
                "Failed to read golden file '{}': {}", 
                file_path.display(), 
                e
            )))?;
        
        // Extract content from golden file format
        self.extract_content_from_golden_file(&content)
    }
    
    fn create_golden_file_content(&self, snapshot: &Snapshot) -> String {
        // Create a golden file with metadata header
        format!(
            "# Golden Snapshot: {}\n# Terminal Size: {}x{}\n# Timestamp: {:?}\n---\n{}",
            snapshot.name,
            snapshot.terminal_size.0,
            snapshot.terminal_size.1,
            snapshot.timestamp,
            snapshot.content
        )
    }
    
    fn extract_content_from_golden_file(&self, golden_file_content: &str) -> Result<String> {
        // Find the separator line "---"
        if let Some(separator_pos) = golden_file_content.find("\n---\n") {
            Ok(golden_file_content[separator_pos + 5..].to_string())
        } else {
            // Old format or malformed file - treat entire content as snapshot data
            Ok(golden_file_content.to_string())
        }
    }
    
    fn compute_diff(&self, expected: &str, actual: &str) -> String {
        // Simple line-by-line diff implementation
        let expected_lines: Vec<&str> = expected.lines().collect();
        let actual_lines: Vec<&str> = actual.lines().collect();
        
        let mut diff_lines = Vec::new();
        let max_lines = expected_lines.len().max(actual_lines.len());
        
        for i in 0..max_lines {
            let expected_line = expected_lines.get(i).unwrap_or(&"");
            let actual_line = actual_lines.get(i).unwrap_or(&"");
            
            if expected_line != actual_line {
                diff_lines.push(format!("  Line {}:", i + 1));
                diff_lines.push(format!("- {}", expected_line));
                diff_lines.push(format!("+ {}", actual_line));
            }
        }
        
        if diff_lines.is_empty() {
            "No differences found".to_string()
        } else {
            format!("Differences found:\n{}", diff_lines.join("\n"))
        }
    }
    
    pub fn get_snapshot_directory(&self) -> &Path {
        &self.snapshot_directory
    }
    
    pub fn is_update_mode(&self) -> bool {
        self.update_mode
    }
    
    pub fn list_golden_files(&self) -> Result<Vec<PathBuf>> {
        let mut golden_files = Vec::new();
        
        if !self.snapshot_directory.exists() {
            return Ok(golden_files);
        }
        
        let entries = fs::read_dir(&self.snapshot_directory)
            .map_err(|e| ComparisonError::IoError(format!(
                "Failed to read snapshot directory '{}': {}", 
                self.snapshot_directory.display(), 
                e
            )))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| ComparisonError::IoError(format!(
                "Failed to read directory entry: {}", e
            )))?;
            
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("golden") {
                golden_files.push(path);
            }
        }
        
        golden_files.sort();
        Ok(golden_files)
    }
    
    pub fn clean_orphaned_golden_files(&self, active_snapshot_names: &[String]) -> Result<Vec<PathBuf>> {
        let golden_files = self.list_golden_files()?;
        let mut cleaned_files = Vec::new();
        
        for golden_file in golden_files {
            if let Some(file_stem) = golden_file.file_stem().and_then(|s| s.to_str()) {
                // Remove .golden extension to get the snapshot name
                let snapshot_name = file_stem.trim_end_matches(".golden");
                
                if !active_snapshot_names.iter().any(|name| {
                    self.sanitize_filename(name) == snapshot_name
                }) {
                    // This golden file is orphaned
                    if self.update_mode {
                        fs::remove_file(&golden_file)
                            .map_err(|e| ComparisonError::IoError(format!(
                                "Failed to remove orphaned golden file '{}': {}", 
                                golden_file.display(), 
                                e
                            )))?;
                        cleaned_files.push(golden_file);
                    }
                }
            }
        }
        
        Ok(cleaned_files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use tempfile::tempdir;
    
    fn create_test_snapshot(name: &str, content: &str) -> Snapshot {
        Snapshot {
            name: name.to_string(),
            content: content.to_string(),
            raw_content: None,
            timestamp: SystemTime::now(),
            terminal_size: (80, 24),
        }
    }
    
    #[test]
    fn test_comparator_creation() {
        let temp_dir = tempdir().unwrap();
        let comparator = SnapshotComparator::new(temp_dir.path().to_path_buf(), false);
        assert!(comparator.is_ok());
        
        let comparator = comparator.unwrap();
        assert_eq!(comparator.get_snapshot_directory(), temp_dir.path());
        assert!(!comparator.is_update_mode());
    }
    
    #[test]
    fn test_filename_sanitization() {
        let temp_dir = tempdir().unwrap();
        let comparator = SnapshotComparator::new(temp_dir.path().to_path_buf(), false).unwrap();
        
        assert_eq!(comparator.sanitize_filename("test/name:with*chars"), "test_name_with_chars");
        assert_eq!(comparator.sanitize_filename("normal_name"), "normal_name");
    }
    
    #[test]
    fn test_golden_file_path() {
        let temp_dir = tempdir().unwrap();
        let comparator = SnapshotComparator::new(temp_dir.path().to_path_buf(), false).unwrap();
        
        let path = comparator.get_golden_file_path("test_snapshot");
        assert_eq!(path, temp_dir.path().join("test_snapshot.golden"));
    }
    
    #[test]
    fn test_golden_file_content_format() {
        let temp_dir = tempdir().unwrap();
        let comparator = SnapshotComparator::new(temp_dir.path().to_path_buf(), false).unwrap();
        
        let snapshot = create_test_snapshot("test", "Hello\nWorld");
        let content = comparator.create_golden_file_content(&snapshot);
        
        assert!(content.contains("# Golden Snapshot: test"));
        assert!(content.contains("# Terminal Size: 80x24"));
        assert!(content.contains("---\n"));
        assert!(content.contains("Hello\nWorld"));
    }
    
    #[test]
    fn test_extract_content_from_golden_file() {
        let temp_dir = tempdir().unwrap();
        let comparator = SnapshotComparator::new(temp_dir.path().to_path_buf(), false).unwrap();
        
        let golden_content = "# Golden Snapshot: test\n# Terminal Size: 80x24\n---\nHello\nWorld";
        let extracted = comparator.extract_content_from_golden_file(golden_content).unwrap();
        assert_eq!(extracted, "Hello\nWorld");
        
        // Test old format without separator
        let old_format = "Hello\nWorld";
        let extracted_old = comparator.extract_content_from_golden_file(old_format).unwrap();
        assert_eq!(extracted_old, "Hello\nWorld");
    }
    
    #[test]
    fn test_simple_diff() {
        let temp_dir = tempdir().unwrap();
        let comparator = SnapshotComparator::new(temp_dir.path().to_path_buf(), false).unwrap();
        
        let expected = "Line 1\nLine 2\nLine 3";
        let actual = "Line 1\nModified Line 2\nLine 3";
        
        let diff = comparator.compute_diff(expected, actual);
        assert!(diff.contains("Line 2:"));
        assert!(diff.contains("- Line 2"));
        assert!(diff.contains("+ Modified Line 2"));
    }
    
    #[test]
    fn test_snapshot_comparison_update_mode() {
        let temp_dir = tempdir().unwrap();
        let comparator = SnapshotComparator::new(temp_dir.path().to_path_buf(), true).unwrap();
        
        let snapshot = create_test_snapshot("test_update", "New content");
        let result = comparator.compare_snapshot(&snapshot).unwrap();
        
        match result.action_taken {
            ComparisonAction::Created => {
                assert!(result.matches);
                assert!(result.golden_file_path.exists());
            },
            _ => panic!("Expected Created action"),
        }
    }
    
    #[test]
    fn test_multiple_snapshots_comparison() {
        let temp_dir = tempdir().unwrap();
        let comparator = SnapshotComparator::new(temp_dir.path().to_path_buf(), true).unwrap();
        
        let snapshots = vec![
            create_test_snapshot("test1", "Content 1"),
            create_test_snapshot("test2", "Content 2"),
        ];
        
        let results = comparator.compare_multiple_snapshots(&snapshots).unwrap();
        assert_eq!(results.len(), 2);
        
        for result in results {
            assert!(result.matches);
            assert!(matches!(result.action_taken, ComparisonAction::Created));
        }
    }
}