#[allow(non_snake_case)]
use structopt::StructOpt;

pub mod clilib;

fn main() {
    let err_catch = clilib::cliargs::Cli::from_args_safe();
    match err_catch {
        Ok(_) => {}
        Err(e) => eprintln!("{}", e),
    }
}
