use crate::app::Cli;
use clap::CommandFactory;
use std::io::{self, Write};

pub fn run() -> io::Result<i32> {
    let mut output = Vec::new();
    clap_mangen::Man::new(Cli::command()).render(&mut output)?;
    io::stdout().write_all(&output)?;
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_manpage() {
        let mut output = Vec::new();
        clap_mangen::Man::new(Cli::command())
            .render(&mut output)
            .expect("manpage");

        let output = String::from_utf8(output).expect("utf8");
        assert!(output.contains(".TH rewrit"));
        assert!(output.contains("Parity engine"));
    }
}
