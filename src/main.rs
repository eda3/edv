use edv::cli;

fn main() {
    // Run the CLI application
    if let Err(err) = cli::run() {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}
