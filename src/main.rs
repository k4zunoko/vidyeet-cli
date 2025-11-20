mod cli;
mod commands;
mod api;
mod config;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    match cli::parse_args(&args) {
        Ok(_) => {},
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
