mod cli;

fn main() {
    if let Err(error) = cli::lib::run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}
