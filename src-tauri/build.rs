fn main() {
    let mut windows = tauri_build::WindowsAttributes::new();
    // Release builds request Administrator (needed for raw sockets). Debug builds don't,
    // so `tauri dev` still works from a normal terminal.
    if std::env::var("PROFILE").as_deref() == Ok("release") {
        windows = windows.app_manifest(include_str!("packaging/windows/app.manifest"));
    }
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("failed to run tauri-build");
}
