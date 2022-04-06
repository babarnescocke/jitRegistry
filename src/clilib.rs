/// CLI module
pub mod cliargs {
    use std::path::PathBuf;
    use structopt::StructOpt;
    #[derive(Debug, StructOpt)]
    #[structopt(
        name = "jitRegistry",
        about = "A container registry-like server",
        author = "Brian A Barnes-Cocke"
    )]
    pub struct Cli {
        ///Directory to serve containers from.
        #[structopt(short, long = "directory", parse(from_os_str), env = "JITREGISTRY_DIR")]
        pub directory_path: PathBuf,
    }

    pub fn cli_return_or_error_exit() -> Cli {
        match Cli::from_args_safe() {
            Ok(x) => x,
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(-1);
            }
        }
    }
}
