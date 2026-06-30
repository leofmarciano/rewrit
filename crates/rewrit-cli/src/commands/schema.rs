use crate::app::SchemaCommand;
use schemars::schema_for;

pub fn run(command: SchemaCommand) -> Result<i32, serde_json::Error> {
    match command {
        SchemaCommand::Export { kind } => {
            let schema = match kind.as_str() {
                "contract" => serde_json::to_value(schema_for!(rewrit_model::Contract))?,
                "observation" => serde_json::to_value(schema_for!(rewrit_model::Observation))?,
                "event" => serde_json::to_value(schema_for!(rewrit_protocol::AdapterEvent))?,
                _ => serde_json::to_value(schema_for!(rewrit_model::Report))?,
            };
            println!("{}", serde_json::to_string_pretty(&schema)?);
            Ok(0)
        }
    }
}
