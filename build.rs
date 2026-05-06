fn main() {
    println!("cargo:rustc-link-lib=gringotts");
    println!("cargo:rerun-if-changed=build.rs");
}
