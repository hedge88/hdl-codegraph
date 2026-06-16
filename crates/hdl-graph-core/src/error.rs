use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Node not found: {0}")]
    NodeNotFound(u64),
    #[error("Edge not found: source={0}, target={1}")]
    EdgeNotFound(u64, u64),
    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),
    #[error("Scope error: {0}")]
    ScopeError(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type CoreResult<T> = Result<T, CoreError>;
