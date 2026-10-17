//! `addr`: a short alias for the `doorplate` command (same code, same usage).
#[path = "../main.rs"]
mod doorplate_main;

fn main() -> std::process::ExitCode {
    doorplate_main::main()
}
