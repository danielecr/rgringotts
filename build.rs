fn main() {
    // Allow pointing cargo at a non-standard install prefix, e.g.:
    //   export LIBGRINGOTTS_DIR=/usr/local    (default after ./setup.sh)
    //   export LIBGRINGOTTS_DIR=$HOME/.local  (user-local install)
    if let Ok(dir) = std::env::var("LIBGRINGOTTS_DIR") {
        println!("cargo:rustc-link-search=native={dir}/lib");
    }
    println!("cargo:rustc-link-lib=gringotts");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=LIBGRINGOTTS_DIR");
}
