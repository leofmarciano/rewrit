use std::io;
use std::path::Path;

pub fn run(template: String) -> io::Result<i32> {
    let manifest = match template.as_str() {
        "laravel-to-encore" => laravel_to_encore_manifest(),
        "django-to-rust" => django_to_rust_manifest(),
        _ => command_to_command_manifest(),
    };

    if !Path::new("rewrit.toml").exists() {
        std::fs::write("rewrit.toml", manifest)?;
    }
    std::fs::create_dir_all("contracts")?;
    std::fs::create_dir_all(".rewrit/reports")?;
    println!("created rewrit.toml using template {template}");
    Ok(0)
}

fn command_to_command_manifest() -> &'static str {
    r#"[project]
name = "rewrit-command-example"
reference = "reference"
candidate = "candidate"
contracts_dir = "contracts"
baselines_dir = ".rewrit/baselines"
reports_dir = ".rewrit/reports"

[runtimes.reference]
adapter = "command"
command = ["./examples/command-to-command/reference.sh"]
timeout_ms = 30000

[runtimes.candidate]
adapter = "command"
command = ["./examples/command-to-command/candidate.sh"]
timeout_ms = 30000

[[reports]]
kind = "terminal"

[[reports]]
kind = "json"
path = ".rewrit/reports/latest.json"
"#
}

fn laravel_to_encore_manifest() -> &'static str {
    r#"[project]
name = "laravel-to-encore"
reference = "reference_laravel"
candidate = "candidate_encore"
contracts_dir = "contracts"
baselines_dir = ".rewrit/baselines"
reports_dir = ".rewrit/reports"

[runtimes.reference_laravel]
adapter = "command"
cwd = "../legacy"
command = ["vendor/bin/pest", "--rewrit"]
timeout_ms = 30000

[runtimes.candidate_encore]
adapter = "command"
cwd = "../candidate"
command = ["npm", "run", "test:rewrit"]
timeout_ms = 30000

[[suites]]
id = "billing"
title = "Billing domain"
policy = "http_api_strict"
required = true

[policies.http_api_strict]
compare_stdout = false
compare_stderr = false
compare_exit_code = true
decimal_as_string = true

[[reports]]
kind = "terminal"

[[reports]]
kind = "json"
path = ".rewrit/reports/latest.json"

[[reports]]
kind = "junit"
path = ".rewrit/reports/junit.xml"
"#
}

fn django_to_rust_manifest() -> &'static str {
    r#"[project]
name = "django-to-rust"
reference = "reference_django"
candidate = "candidate_rust"
contracts_dir = "contracts"
baselines_dir = ".rewrit/baselines"
reports_dir = ".rewrit/reports"

[runtimes.reference_django]
adapter = "command"
command = ["pytest", "--rewrit"]
timeout_ms = 30000

[runtimes.candidate_rust]
adapter = "command"
command = ["cargo", "test", "--", "--rewrit"]
timeout_ms = 30000

[[reports]]
kind = "terminal"

[[reports]]
kind = "json"
path = ".rewrit/reports/latest.json"
"#
}

