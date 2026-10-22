fn main() {
    // Embed the app icon (dark tile) into the exe: Explorer, taskbar, Alt-Tab.
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/logo/icon-dark.ico");
        let mut res = winresource::WindowsResource::new();
        res.set_icon_with_id("assets/logo/icon-dark.ico", "1");
        res.compile().expect("failed to embed app icon");
    }
}
