//! `addr` command-line tool (skeleton). Planned subcommands: `parse`,
//! `parse-file`, `corpus test`.

use addr_core::Parser;

fn main() {
    let mut args = std::env::args().skip(1);
    match (args.next().as_deref(), args.next()) {
        (Some("parse"), Some(input)) => match Parser::new().parse(&input) {
            Ok(a) => println!("{a:#?}"),
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(2);
            }
        },
        _ => {
            eprintln!("usage: addr parse \"<address>\"");
            std::process::exit(1);
        }
    }
}
