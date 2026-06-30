use crate::app::ReportCommand;
use std::io;

pub fn run(command: ReportCommand) -> io::Result<i32> {
    match command {
        ReportCommand::Open { path } => {
            let contents = std::fs::read_to_string(path)?;
            println!("{contents}");
            Ok(0)
        }
    }
}
