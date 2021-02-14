use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let library = pkg_config::Config::new().probe("flac").unwrap();
    let profile = env::var_os("PROFILE").unwrap();
    let from = library.link_paths.get(0).unwrap().join("libFLAC.so");
    let to = Path::new("target").join(profile).join("deps").join("libflac.so");
    fs::copy(from, to).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
}
