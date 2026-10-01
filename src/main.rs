// Added by the HAT Specifications project, 2026.
// Purpose: expose a debug-only protocol/schema graph projection CLI.

fn main() {
    if let Err(error) = hat_protocol_schema_debugger::run_cli(std::env::args()) {
        eprintln!("{error}");
        std::process::exit(2);
    }
}
