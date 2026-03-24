/// CLI module
pub mod cliargs {
    use crate::buildah::b;
    use actix_web::web;
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

        /// Directory for OCI layout cache (where built images are exported for serving).
        #[structopt(
            short = "C",
            long = "cache-dir",
            parse(from_os_str),
            env = "JITREGISTRY_CACHE_DIR"
        )]
        pub cache_dir: Option<PathBuf>,
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
    #[derive(Debug, Clone)]
    pub struct Args {
        pub con_dir_path: PathBuf,
        pub bind_addr: Ipv4Addr,
        pub bind_port: u16,
        pub buildah_dir: PathBuf,
        pub oci_cache_dir: PathBuf,
    }

    impl Args {
        /// Pretty early on I want to exit if there is an error; This makes a new Args or exits on error.
        pub fn args_or_exit() -> Self {
            let ui = new_Cli_or_exit();
            match b::buildah_graphroot() {
                Ok(x) => {
                    let cache_dir = ui.cache_dir.unwrap_or_else(|| {
                        let mut cd = x.clone();
                        cd.pop(); // go up from graphroot
                        cd.push("jitregistry-cache");
                        cd
                    });
                    std::fs::create_dir_all(&cache_dir).unwrap_or_else(|e| {
                        eprintln!("Cannot create cache directory {:?}: {}", cache_dir, e);
                        std::process::exit(-3);
                    });
                    Args {
                        con_dir_path: ui.directory_path,
                        bind_addr: ui.bind_addr,
                        bind_port: ui.bind_port,
                        buildah_dir: x,
                        oci_cache_dir: cache_dir,
                    }
                }
                Err(e) => {
                    eprintln!("This is a high-level program error, no service has started. \n Error:{} \n Exiting...", e);
                    std::process::exit(-2);
                }
            }
        }
        pub fn args_to_data_wa(&self) -> web::Data<WA> {
            web::Data::new(WA::new(
                self.con_dir_path.clone(),
                self.buildah_dir.clone(),
                self.oci_cache_dir.clone(),
            ))
        }
    }

    #[derive(Debug, Clone)]
    pub struct WA {
        pub con_dir_path: PathBuf,
        pub buildah_dir: PathBuf,
        pub oci_cache_dir: PathBuf,
    }
    impl WA {
        pub fn new(cdp: PathBuf, bp: PathBuf, ocd: PathBuf) -> Self {
            WA {
                con_dir_path: cdp,
                buildah_dir: bp,
                oci_cache_dir: ocd,
            }
        }
    }
}
