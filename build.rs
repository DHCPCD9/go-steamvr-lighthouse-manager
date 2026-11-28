fn main() {
    println!("cargo:rerun-if-changed=resources/icon.ico");
    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("resources/icon.ico");
        res.set("ProductName", "SteamVR Lighthouse Manager");
        res.set("FileDescription", "SteamVR Lighthouse Manager");
        res.compile().expect("embed Windows application icon");
    }
}
