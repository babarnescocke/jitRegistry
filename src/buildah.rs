pub mod b {
    use serde_json::{from_str, Value};
    use std::path::PathBuf;
    use std::process::Command;
    /// Posix compliant `command -v` to find buildah in path. Ideally, we would like users to specify path as well.
    /// I am not sure why but, command -v fails without sh -c.
    pub fn buildah_command_in_path() -> Result<bool, Box<dyn std::error::Error>> {
        let exit_code_on_finding_buildah = Command::new("sh")
            .arg("-c")
            .arg("command")
            .arg("-v")
            .arg("buildah")
            .output();
        match exit_code_on_finding_buildah {
            Ok(code) => {
                if code.status.code().unwrap() == 0 {
                    return Ok(true);
                }
            }
            Err(_) => Err("Unix systems can only reach this error by signal interrupt. Error")?,
        }
        Ok(false)
    }
    pub fn buildah_unshare_build(path: &PathBuf) -> Result<String, Box<dyn std::error::Error>> {
        let output = Command::new("sh")
            .arg("-c")
            .arg("buildah")
            .arg("unshare")
            .arg(path.canonicalize()?.to_str().unwrap())
            .output()?;
        if output.status.code().unwrap() == 0 {
            Ok(vec_u8_to_last_line(&output.stdout[..]))
        } else {
            Err(format!("buildah error"))?
        }
    }
    /// Returns buildah's graphroot (where Build keeps relevant images, manifest) from `buildah info`. The performance characteristics are pretty bad, .07s to execute, vs roughly .03 for 'buildah info'. Needed to reliably return find image layers and manifests.
    // First we walk execute `sh -c buildah info` which gives the user-specific JSON response to how buildah is configured. (e.g. if this is run with different users, you will get different values.)
    // We only care about the json attribute at store.GraphRoot. We have to process the Result<Output>. I haven't tested but I think an error should be an exit condition...
    // stderr and stdout are Vec[u8]s and so we package them as strings, and concat them onto an error string, to be returned on error. But, if we can parse the returned vec[u8] to JSON as a str, read into the store object for GraphRoot,
    // and create a PathBuf, we will return that.
    pub fn buildah_graphroot() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let output = Command::new("sh").arg("-c").arg("buildah info").output()?;
        let mut error_string = String::from("");
        if output.status.code().is_some() {
            // *nix systems a 0 exit code is success, so we don't care about that condition.
            if output.status.code().unwrap() != 0 {
                error_string.push_str(&format!(
                    "buildah Error code: {}",
                    output.status.code().unwrap()
                ));
            }
        }
        if output.stderr.len() > 0 {
            error_string
                .push_str(&(String::from("\n") + &std::str::from_utf8(&output.stderr)?.to_owned()));
        }
        if output.stdout.len() > 0 {
            let res: Result<Value, serde_json::Error> =
                from_str(&std::str::from_utf8(&output.stdout)?);
            match res {
                Ok(x) => {
                    let str_wo_end_quote = x["store"]["GraphRoot"].as_str().unwrap().trim_end();
                    let str_wo_start_quote = str_wo_end_quote.trim_start();
                    return Ok(PathBuf::from(str_wo_start_quote));
                }
                Err(e) => {
                    error_string.push_str(&(String::from("\n") + &e.to_string().to_owned()));
                }
            }
        }
        Err(error_string)?
    }

    fn vec_u8_to_last_line(v: &[u8]) -> String {
        //let mut index: usize = 0;
        //for (i, y) in v.iter().enumerate() {
        //    if (*y).contains(b"\n") {
        //        index = i;
        //    }
        //    let (_, last_line) = v.split_at(index);
        String::from("not_string")
        //}
    }
    /*/// To help delineate options and settings.
    pub struct BuildahOptions {
        build_type: BuildBuildFromTargets,
    }
    /// Enum of different build options.
    pub enum BuildahBuildFromTargets {
        Oci,
        Dockerfile,
    }*/
}
