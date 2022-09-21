use cc::Build;
use std::{env, error::Error, fs::File, io::Write, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    // build directory for this crate
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    // extend the library search path
    println!("cargo:rustc-link-search={}", out_dir.display());

    File::create(out_dir.join("htvmm.lds"))?.write_all(include_bytes!("src/htvmm.lds"))?;

    // assemble the `asm.s` file
    Build::new().file("entry.s").compile("entry");

    // rebuild if `entry.s` changed
    println!("cargo:rerun-if-changed=entry.s");

    Ok(())
}
