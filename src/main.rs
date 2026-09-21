use owo_colors::OwoColorize;

use fastgen_cli::cli;

fn main() {
    if let Err(err) = cli::main() {
        anstream::eprintln!("{} {err:#}", "error:".red());
        std::process::exit(1);
    }
}
