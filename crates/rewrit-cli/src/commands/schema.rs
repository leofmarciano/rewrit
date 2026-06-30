use crate::app::SchemaCommand;
use schemars::schema_for;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SchemaExportError {
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
struct NamedSchema {
    name: &'static str,
    file_name: &'static str,
    value: Value,
}

pub fn run(command: SchemaCommand) -> Result<i32, SchemaExportError> {
    match command {
        SchemaCommand::Export { kind, out_dir } => {
            let schemas = schemas_for(&kind)?;
            if let Some(out_dir) = out_dir {
                write_schemas(&out_dir, &schemas)?;
            } else if let [schema] = schemas.as_slice() {
                println!("{}", serde_json::to_string_pretty(&schema.value)?);
            } else {
                let values = schemas
                    .into_iter()
                    .map(|schema| (schema.name.to_string(), schema.value))
                    .collect::<BTreeMap<_, _>>();
                println!("{}", serde_json::to_string_pretty(&values)?);
            }
            Ok(0)
        }
    }
}

fn schemas_for(kind: &str) -> Result<Vec<NamedSchema>, serde_json::Error> {
    if kind == "all" {
        return [
            "contract",
            "observation",
            "event",
            "adapter_request",
            "report",
        ]
        .into_iter()
        .map(schema_for_kind)
        .collect();
    }

    schema_for_kind(kind).map(|schema| vec![schema])
}

fn schema_for_kind(kind: &str) -> Result<NamedSchema, serde_json::Error> {
    match kind {
        "contract" => Ok(NamedSchema {
            name: "contract",
            file_name: "rewrit.contract.v1.schema.json",
            value: serde_json::to_value(schema_for!(rewrit_model::Contract))?,
        }),
        "observation" => Ok(NamedSchema {
            name: "observation",
            file_name: "rewrit.observation.v1.schema.json",
            value: serde_json::to_value(schema_for!(rewrit_model::Observation))?,
        }),
        "event" => Ok(NamedSchema {
            name: "event",
            file_name: "rewrit.event.v1.schema.json",
            value: serde_json::to_value(schema_for!(rewrit_protocol::AdapterEvent))?,
        }),
        "adapter_request" | "adapter-request" => Ok(NamedSchema {
            name: "adapter_request",
            file_name: "rewrit.adapter_request.v1.schema.json",
            value: serde_json::to_value(schema_for!(rewrit_protocol::AdapterRequest))?,
        }),
        _ => Ok(NamedSchema {
            name: "report",
            file_name: "rewrit.report.v1.schema.json",
            value: serde_json::to_value(schema_for!(rewrit_model::Report))?,
        }),
    }
}

fn write_schemas(out_dir: &Path, schemas: &[NamedSchema]) -> Result<(), SchemaExportError> {
    std::fs::create_dir_all(out_dir)?;
    for schema in schemas {
        let path: PathBuf = out_dir.join(schema.file_name);
        let encoded = serde_json::to_vec_pretty(&schema.value)?;
        std::fs::write(&path, encoded)?;
        println!("{}", path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_exports_protocol_and_report_schemas() {
        let temp = tempfile::tempdir().expect("tempdir");

        run(SchemaCommand::Export {
            kind: "all".to_string(),
            out_dir: Some(temp.path().to_path_buf()),
        })
        .expect("schema export");

        assert!(temp
            .path()
            .join("rewrit.adapter_request.v1.schema.json")
            .is_file());
        assert!(temp.path().join("rewrit.event.v1.schema.json").is_file());
        assert!(temp.path().join("rewrit.report.v1.schema.json").is_file());
    }
}
