use crate::app::Cli;
use clap::CommandFactory;
use clap_complete::Shell;
use std::io;

pub fn run(shell: Shell) -> io::Result<i32> {
    let mut command = Cli::command();
    clap_complete::generate(shell, &mut command, "rewrit", &mut io::stdout());
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_bash_completions() {
        let mut command = Cli::command();
        let mut output = Vec::new();
        clap_complete::generate(Shell::Bash, &mut command, "rewrit", &mut output);

        let output = String::from_utf8(output).expect("utf8");
        assert!(output.contains("_rewrit"));
        assert!(output.contains("completions"));
    }
}
