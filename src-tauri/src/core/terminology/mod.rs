//! Global, multilingual terminology library.
//!
//! The core owns language-neutral persistence and normalization. Engine-owned
//! semantic classification and language-specific morphology live behind
//! dedicated adapters and never leak into this module's repository queries.

pub mod normalize;
pub mod repository;
pub mod types;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TerminologyError {
    #[error("invalid terminology input: {0}")]
    InvalidInput(String),
    #[error("terminology entry not found: {0}")]
    NotFound(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

pub type Result<T> = std::result::Result<T, TerminologyError>;
