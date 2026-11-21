fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rerun-if-changed=native/calendar.m");
        println!("cargo:rerun-if-changed=native/floating.m");
        println!("cargo:rerun-if-changed=native/agents.m");
        println!("cargo:rerun-if-changed=native/dictation.m");
        println!("cargo:rerun-if-changed=native/dictation.swift");
        cc::Build::new()
            .file("native/calendar.m")
            .file("native/floating.m")
            .file("native/agents.m")
            .file("native/dictation.m")
            .flag("-fobjc-arc")
            .flag("-fblocks")
            .flag("-Wno-deprecated-declarations")
            .compile("duck_eventkit");
        println!("cargo:rustc-link-lib=framework=EventKit");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=AVFoundation");
        println!("cargo:rustc-link-lib=framework=Speech");
        build_swift_dictation();
    }
    tauri_build::build()
}

fn build_swift_dictation() {
    use std::{env, path::PathBuf, process::Command};

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("Falta OUT_DIR"));
    let arch = match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86_64") => "x86_64",
        _ => panic!("Arquitectura macOS no compatible con el dictado"),
    };
    let target = format!("{arch}-apple-macosx13.0");
    let sdk = Command::new("xcrun")
        .args(["--sdk", "macosx", "--show-sdk-path"])
        .output().expect("No se encontró el SDK de macOS");
    assert!(sdk.status.success(), "No se pudo localizar el SDK de macOS");
    let sdk = String::from_utf8(sdk.stdout).expect("Ruta del SDK inválida");
    let sdk = PathBuf::from(sdk.trim());
    let status = Command::new("xcrun")
        .args(["--sdk", "macosx", "swiftc", "-parse-as-library", "-emit-library", "-static", "-O"])
        .args(["-module-name", "DuckDictation", "-target", &target])
        .arg("-sdk").arg(&sdk)
        .arg("-module-cache-path").arg(out.join("swift-cache"))
        .arg("native/dictation.swift")
        .arg("-o").arg(out.join("libduck_dictation.a"))
        .status().expect("No se encontró el compilador Swift");
    assert!(status.success(), "No se pudo compilar el dictado con el SDK de macOS 26");
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-search=native={}", sdk.join("usr/lib/swift").display());
    println!("cargo:rustc-link-lib=static=duck_dictation");
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
}
