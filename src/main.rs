use std::process::ExitCode;

use vitals::cli::{self, Command};

fn main() -> ExitCode {
    match cli::parse(std::env::args().skip(1)) {
        Ok(Command::Run(options)) => {
            println!("{}", vitals::line(&options));
            ExitCode::SUCCESS
        }
        Ok(Command::Help) => {
            print!("{}", cli::HELP);
            ExitCode::SUCCESS
        }
        Ok(Command::Version) => {
            println!("vitals {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("vitals: {message}\nTry 'vitals --help'.");
            ExitCode::from(2)
        }
    }
}
