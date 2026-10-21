fn main() {
    // Embed the app icon into the exe (Explorer, taskbar, Alt-Tab).
    // ID 1 (dark tile) is the exe's default icon; ID 2 (light tile) is
    // loaded at runtime for dark mode.
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/logo/icon-dark.ico");
        println!("cargo:rerun-if-changed=assets/logo/icon-light.ico");
        let mut res = winresource::WindowsResource::new();
        res.set_icon_with_id("assets/logo/icon-dark.ico", "1");
        res.set_icon_with_id("assets/logo/icon-light.ico", "2");
        res.compile().expect("failed to embed app icon");
    }
}
