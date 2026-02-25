use std::env;

fn main() {
    // Don't link the default CRT
    // Link the debug CRT instead
    cxx_build::bridge("src/lib.rs")
        .flag_if_supported("-std=c++17")
        .flag_if_supported("/std:c++17")
        .flag_if_supported("/Zc:__cplusplus")
        .compile("oneiroi_cpp");

    println!("cargo:rerun-if-changed=src/lib.rs");
    //println!("cargo::rustc-link-arg=/nodefaultlib:libcmt");
    //println!("cargo::rustc-link-arg=/defaultlib:libcmt");

    if env::var("TARGET").is_ok_and(|s| s.contains("windows-msvc")) {
        // MSVC compiler suite
        if env::var("CFLAGS").is_ok_and(|s| s.contains("/MDd"))
            || Ok("debug".to_owned()) == env::var("PROFILE")
        {
            //if Ok("debug".to_owned()) == env::var("PROFILE") {
            // debug runtime flag is set

            // Don't link the default CRT
            println!("cargo::rustc-link-arg=/nodefaultlib:msvcrt");
            // println!("cargo::rustc-link-arg=/nodefaultlib:libcmt");
            // Link the debug CRT instead
            println!("cargo::rustc-link-arg=/defaultlib:msvcrtd");

            //println!("cargo::rustc-link-arg-bins=/nodefaultlib:msvcrt");
            // Link the debug CRT instead
            //println!("cargo::rustc-link-arg-bins=/defaultlib:msvcrtd");
        }
    }
}
