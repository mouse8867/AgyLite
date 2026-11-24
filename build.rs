fn main() {
    // Embed the app icon (and version strings) into the Windows executable.
    // Failure is non-fatal: the app still builds and runs without it.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "AgyLite");
        res.set(
            "FileDescription",
            "AgyLite - Antigravity No-TUN Launcher",
        );
        if let Err(e) = res.compile() {
            eprintln!("warning: could not embed icon: {e}");
        }
    }
}
