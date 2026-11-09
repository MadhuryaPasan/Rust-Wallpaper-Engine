// Shared by both executable build scripts. Cargo supplies the inherited workspace
// version, so Explorer's Details tab cannot drift from the installer or About page.
fn main() {
    println!("cargo:rerun-if-changed=../../scripts/windows-resource.rs");
    println!("cargo:rerun-if-changed=../../Cargo.toml");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let package = std::env::var("CARGO_PKG_NAME").expect("Cargo package name");
    let mut resource = winresource::WindowsResource::new();
    resource
        .set("ProductName", "Serein")
        .set("FileDescription", if package == "serein-studio" { "Serein Studio" } else { "Serein desktop player" })
        .set("OriginalFilename", &format!("{package}.exe"));
    resource.compile().expect("Unable to compile Windows version resources");
}
