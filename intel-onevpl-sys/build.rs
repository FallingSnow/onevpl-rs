use std::{env, path::PathBuf};

fn main() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let lib_vpl_include_path = env::var("LIBVPL_INCLUDE_PATH");

    println!("cargo:rustc-link-lib=dylib=vpl");

    let libvpl_include_path = match lib_vpl_include_path {
        Ok(path) => PathBuf::from(path),
        _ => {
            #[cfg(not(target_os = "windows"))]
            {
                // https://github.com/Intel-Media-SDK/MediaSDK/blob/master/api/include/mfxvideo.h
                // https://rust-lang.github.io/rust-bindgen/tutorial-3.html
                let libvpl = pkg_config::probe_library("vpl").unwrap();
                libvpl.include_paths[0].join("vpl")
            }
            #[cfg(target_os = "windows")]
            PathBuf::from("C:/Program Files (x86)/vpl/include/vpl/")
        }
    };
    

    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header(libvpl_include_path.join("mfx.h").to_string_lossy())
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .dynamic_library_name("vpl")
        .derive_debug(true)
        .impl_debug(true)
        // https://github.com/rust-lang/rust-bindgen/issues/2221
        .no_debug("mfx3DLutSystemBuffer")
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // #[cfg(feature = "va")]
    // {
    //     println!("cargo:rustc-link-lib=dylib=va-drm");
    //     let libvadrm = pkg_config::probe_library("libva-drm").unwrap();
    //     let libvadrm_include_path = libvadrm.include_paths[0].join("va");
    //     let bindings = bindgen::Builder::default()
    //         .header(libvadrm_include_path.join("va_drm.h").to_string_lossy())
    //         .parse_callbacks(Box::new(bindgen::CargoCallbacks))
    //         .generate()
    //         .expect("Unable to generate bindings");

    //     bindings
    //         .write_to_file(out_path.join("bindings_va.rs"))
    //         .expect("Couldn't write bindings!");
    // }
}
