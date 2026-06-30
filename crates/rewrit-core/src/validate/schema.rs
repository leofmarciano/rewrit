use rewrit_model::Contract;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SchemaValidationError {
    #[error("unsupported contract schema_version: {0}")]
    UnsupportedVersion(String),
}

pub fn validate_contract(contract: &Contract) -> Result<(), SchemaValidationError> {
    if contract.schema_version != "rewrit.contract.v1" {
        return Err(SchemaValidationError::UnsupportedVersion(
            contract.schema_version.clone(),
        ));
    }
    Ok(())
}
