fn main() {
    tauri_build::build();
    embed_resource::compile("resource.rc", embed_resource::NONE);
}
