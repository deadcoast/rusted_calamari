fn main() {
    println!("cargo:rerun-if-changed=assets/icons/rusted-calamari.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/icons/rusted-calamari.ico")
            .set("ProductName", "Rusted Calamari")
            .set(
                "FileDescription",
                "Rusted Calamari image browser and organiser",
            )
            .set("OriginalFilename", "rusted-calamari.exe")
            .compile()
            .expect("Compile the Windows icon and version resources");
    }
}
