pub mod b {
    use oci_spec::image::ImageManifest;
    use serde_json::{from_str, Value};
    use std::path::PathBuf;
    use std::process::Command;
    use walkdir::WalkDir;

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
            .arg(format!(
                "buildah unshare {}",
                path.canonicalize()?.to_str().unwrap()
            ))
            .output()?;
        if output.status.code().unwrap() == 0 {
            Ok(vec_u8_to_last_line(&output.stdout)?)
        } else {
            Err(format!("buildah error"))?
        }
    }
    pub fn buildah_dockerconatinerfile_build(
        path: &PathBuf,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let output = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "buildah bud -t {}",
                path.canonicalize()?.to_str().unwrap()
            ))
            .output()?;
        if output.status.code().unwrap() == 0 {
            Ok(vec_u8_to_last_line(&output.stdout[..])?.to_string())
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

    fn vec_u8_to_last_line(v: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
        let zero = std::str::from_utf8(v)?;
        let lines = zero.lines();
        let mut tstring: String = String::from("");
        for l in lines {
            tstring = l.to_string();
        }
        Ok(tstring)
    }

    /// At time of this writing, pathbuf doesn't have a try_exists in stable -so we are just going to use exists for now, it obviously flattens errors
    fn PathBuf_has_sub_dir(pb: &mut PathBuf, sd: &str) -> bool {
        pb.push(sd);
        pb.is_dir()
    }

    /// takes a PathBuf and says we either have an executable buildah script script, dockerfile/containerfile or an error
    pub fn pathbuf_to_actionable_buildah_path(
        pb: &PathBuf,
        sd: &str,
    ) -> Result<(Option<PathBuf>, Option<PathBuf>), Box<dyn std::error::Error>> {
        if PathBuf_has_sub_dir(&mut (pb.clone()), sd.clone()) {
            let mut path = pb.clone();
            path.push(sd);
            for f in WalkDir::new(path.clone()).max_depth(1) {
                let F = f?;
                let E = F.clone();
                let metadata = (F.clone()).metadata()?;
                if metadata.is_file() {
                    let testr: &str = E.file_name().to_str().unwrap();
                    let testrclone0 = testr.clone();
                    let testrclone1 = testr.clone();
                    if testrclone0.starts_with("Dockerfile")
                        || testrclone1.starts_with("Containerfile")
                    {
                        return Ok((None, Some(path)));
                    } else if testr.ends_with("sh") {
                        return Ok((Some(F.into_path()), None));
                    }
                }
            }
        }
        Err(format!("Cannot find a subdirectory for path: {:?}", pb))?
    }

    pub fn hash_to_manifest(
        h: &str,
        bp: &mut PathBuf,
    ) -> Result<ImageManifest, Box<dyn std::error::Error>> {
        Ok(ImageManifest::from_file(PathBuf::from(format!(
            "{}/overlay-images/{}/manifest",
            bp.to_string_lossy(),
            h
        )))?)
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
