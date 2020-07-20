use std::process;

mod cli;

fn main() {
    match cli::run() {
        Ok(_) => (),
        Err(error) => {
            eprintln!("{}", error);
            process::exit(1);
        }
    }
}
