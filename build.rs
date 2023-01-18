use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let library = pkg_config::Config::new().probe("flac").unwrap();
    let profile = env::var_os("PROFILE").unwrap();

    let lib_flac_from = String::from(
        if env::consts::OS == "macos"{
            "libFLAC.dylib"
        }
        else{
            "libFLAC.so"
        }
    );
    let lib_flac_to = String::from(
        if env::consts::OS == "macos"{
            "libFLAC.dylib"
        }
        else{
            "libflac.so"
        }
    );
    let from = library.link_paths.get(0).unwrap().join(lib_flac_from);
    let to = Path::new("target").join(profile).join("deps").join(lib_flac_to);
    fs::copy(from, to).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
}
