/// CLI module
pub mod cliargs {
    //use crate::buildah::b;
    use std::net::Ipv4Addr;
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

        /// Bind IPv4 address.
        #[structopt(
            short,
            long = "bind-address",
            parse(try_from_str),
            default_value = "127.0.0.1",
            env = "JITREGISTRY_BIND_ADDR"
        )]
        pub bind_addr: Ipv4Addr,

        ///Bind port, 0-65535.
        #[structopt(
            short = "B",
            long = "bind-port",
            default_value = "7999",
            env = "JITREGISTRY_BIND_PORT"
        )]
        pub bind_port: u16,
    }

    impl Cli {
        /*        fn dir_path_to_sub_dir_vec(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
            if self.directory_path.is_dir() {
                std::fs::read_dir(self.directory_path)
                    .into_iter()
                    .filter(|e| subdir_is_readable_contains_suitable(e))
                    .collect()
            } else {
                Err(format!(
                    "directory_path: {:?}, is not a directory",
                    self.directory_path.clone()
                ))?
            }
        }*/
        fn dont_error_out(&self) -> Result<bool, Box<dyn std::error::Error>> {
            Ok(true)
        }
    }
    /// This wraps the Cli struct and produces Cli, or it will exit the program, with the very helpful structopt error message.
    pub fn cli_return_or_error_exit() -> Cli {
        match Cli::from_args_safe() {
            Ok(x) => match x.dont_error_out() {
                Ok(dont_stop_bool) => {
                    if dont_stop_bool {
                        x
                    } else {
                        std::process::exit(-1)
                    }
                }
                Err(e) => {
                    eprintln!("This is a high-level program error, no service has started. \n Error:{} \n Exiting...", e);
                    std::process::exit(-1);
                }
            },
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(-1);
            }
        }
    }
}
