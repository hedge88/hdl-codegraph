use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceSpan {
    pub file_id: u64,
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl SourceSpan {
    pub fn new(file_id: u64, sl: u32, sc: u32, el: u32, ec: u32) -> Self {
        Self { file_id, start_line: sl, start_col: sc, end_line: el, end_col: ec }
    }
}
