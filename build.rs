fn main() {
    println!("cargo:rerun-if-changed=assets/icons/devkits.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/icons/devkits.ico")
            .set("ProductName", "DevKits")
            .set("FileDescription", "DevKits developer toolbox")
            .compile()
            .expect("compile DevKits Windows icon and version resources");
    }
}
