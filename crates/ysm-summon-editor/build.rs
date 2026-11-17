//! Embeds the application icon and version metadata into the Windows executable.

fn main() {
    println!("cargo:rerun-if-changed=../../assets/Logo.ico");

    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("../../assets/Logo.ico");
        resource.set("ProductName", "YSM Summon Editor");
        resource.set("FileDescription", "YSM Summon Editor");
        if let Err(error) = resource.compile() {
            // A missing resource compiler should not block the build; the app just has no exe icon.
            println!("cargo:warning=could not embed the Windows icon: {error}");
        }
    }
}
