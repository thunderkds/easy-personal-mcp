// T016: capabilities are compiled in. By default the window gets none (the page is a remote origin).
// `T016_REMOTE_IPC=1 cargo build` compiles `capabilities-remote/remote.json` in, to see what a remote origin needs for IPC.
fn main() {
    println!("cargo:rerun-if-env-changed=T016_REMOTE_IPC");
    let mut attrs = tauri_build::Attributes::new();
    if std::env::var_os("T016_REMOTE_IPC").is_some() {
        attrs = attrs.capabilities_path_pattern("./capabilities-remote/*.json");
    } else {
        attrs = attrs.capabilities_path_pattern("./no-capabilities/*.json");
    }
    tauri_build::try_build(attrs).expect("tauri build failed");
}
