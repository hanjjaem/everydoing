fn main() {
    tauri_build::build();
    // UAC requireAdministrator — MSVC linker manifest injection
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!(r#"cargo:rustc-link-arg=/MANIFESTUAC:level='requireAdministrator' uiAccess='false'"#);
    }
}
