use thiserror::Error;

#[derive(Debug, Error)]
pub enum ManifestValidationError {
    #[error("manifest has no runtimes")]
    NoRuntimes,
    #[error("manifest has no project name")]
    MissingProjectName,
}
