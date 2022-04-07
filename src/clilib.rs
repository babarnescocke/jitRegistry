/// CLI module
pub mod cliargs {
    use crate::buildah::b;
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
    /// Produces a Cli struct or exits. The errors from structopt are very informative, so they get passed completely.
    pub fn new_Cli_or_exit() -> Cli {
        match Cli::from_args_safe() {
            Ok(x) => x,
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(-1);
            }
        }
    }

    /// Will be used to sanitize Cli, instantiate a couple things early in runtime and one day could integrate a configuration file(s).
    #[derive(Debug)]
    pub struct Args {
        pub con_dir_path: PathBuf,
        pub bind_addr: Ipv4Addr,
        pub bind_port: u16,
        pub buildah_dir: PathBuf,
    }

    impl Args {
        pub fn args_or_exit() -> Self {
            let ui = new_Cli_or_exit();
            match b::buildah_graphroot() {
                Ok(x) => Args {
                    con_dir_path: ui.directory_path,
                    bind_addr: ui.bind_addr,
                    bind_port: ui.bind_port,
                    buildah_dir: x,
                },
                Err(e) => {
                    eprintln!("This is a high-level program error, no service has started. \n Error:{} \n Exiting...", e);
                    std::process::exit(-2);
                }
            }
        }
    }
}
