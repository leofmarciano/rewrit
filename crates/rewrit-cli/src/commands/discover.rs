use rewrit_model::Case;

pub fn print_cases(cases: &[Case], format: &str) -> Result<(), serde_json::Error> {
    if format == "json" {
        println!("{}", serde_json::to_string_pretty(cases)?);
    } else {
        for case in cases {
            println!("{}\t{}", case.id, case.title);
        }
    }
    Ok(())
}
