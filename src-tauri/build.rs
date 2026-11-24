fn main() {
    // The icons are compiled into the binary (exe resource, window icon and the
    // tray's include_image!), so a new icon must trigger a rebuild; cargo does
    // not track those files on its own.
    println!("cargo:rerun-if-changed=icons");
    tauri_build::build()
}
