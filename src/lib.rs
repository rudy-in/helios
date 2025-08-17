use std::collections::HashMap;
use std::fs;
use walkdir::WalkDir;

pub struct SearchEngine {
    index: HashMap<String, Vec<String>>,
}

impl SearchEngine {
    pub fn new(path: &str) -> Self {
        let mut index: HashMap<String, Vec<String>> = HashMap::new();

        let mut files: Vec<_> = WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_path_buf())
            .collect();

        files.sort();

        for file_path in files {
            if file_path.is_file() {
                if let Ok(contents) = fs::read_to_string(&file_path) {
                    let file_name = file_path.display().to_string();
                    for word in contents.split_whitespace() {
                        let clean = word
                            .trim_matches(|c: char| !c.is_alphanumeric())
                            .to_lowercase();
                        if !clean.is_empty() {
                            index.entry(clean).or_default().push(file_name.clone());
                        }
                    }
                }
            }
        }

        Self { index }
    }

    pub fn search(&self, query: &str) -> Vec<String> {
        let q = query.to_lowercase();
        if let Some(files) = self.index.get(&q) {
            files
                .iter()
                .map(|f| format!("'{}' found in {}", q, f))
                .collect()
        } else {
            vec![format!("'{}' not found in any file", q)]
        }
    }
}
