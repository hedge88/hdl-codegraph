use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub project: ProjectSection,
    pub index: IndexSection,
    pub search: SearchSection,
    pub lsp: LspSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSection {
    pub name: String,
    pub root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexSection {
    pub include_dirs: Vec<String>,
    pub uvm_home: Option<String>,
    pub defines: Vec<String>,
    pub jobs: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSection {
    pub max_results: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspSection {
    pub port: u16,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            project: ProjectSection {
                name: String::new(),
                root: ".".to_string(),
            },
            index: IndexSection {
                include_dirs: vec!["rtl".into(), "tb".into(), "sim".into()],
                uvm_home: None,
                defines: vec![],
                jobs: 4,
            },
            search: SearchSection { max_results: 100 },
            lsp: LspSection { port: 8300 },
        }
    }
}
