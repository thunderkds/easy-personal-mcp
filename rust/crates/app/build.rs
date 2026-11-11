//! Build script. `tauri_build` runs only for `--features window` (T023): a default build links no Tauri code.
//! The window gets no capability: the pattern below names a folder that does not exist and must stay without
//! `*.json` files, so a page from the loopback origin can call no Tauri command (spike-T016.md, answer 4).

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(feature = "window")]
    {
        let attributes =
            tauri_build::Attributes::new().capabilities_path_pattern("./no-capabilities/*.json");
        tauri_build::try_build(attributes).expect("tauri build failed");
    }
}
