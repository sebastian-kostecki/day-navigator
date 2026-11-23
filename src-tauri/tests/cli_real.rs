//! Manual smoke test against the installed `glab` / `gh`. Prints only outcome
//! kinds, never output. Run: `cargo test --test cli_real -- --ignored --nocapture`.

use day_navigator_lib::cli::{CliCall, CliRunner};

#[test]
#[ignore = "manual: runs the real glab and gh"]
fn real_clis_report_outcome_kinds() {
    let runner = CliRunner::system();
    for call in CliCall::ALL {
        let kind = match runner.run(call) {
            Ok(_) => "data",
            Err(e) => e.kind(),
        };
        println!("{} -> {kind}", call.name());
    }
}
