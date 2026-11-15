//! Incrusta el icono y los metadatos en el ejecutable de Windows.

fn main() {
    println!("cargo:rerun-if-changed=assets/icons/icon.ico");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icons/icon.ico");
        res.set("ProductName", "NovaFlix");
        res.set("FileDescription", "NovaFlix");
        res.set("CompanyName", "Novatic");
        res.set("LegalCopyright", "Novatic · PolyForm Noncommercial 1.0.0");
        if let Err(err) = res.compile() {
            println!("cargo:warning=no se pudo incrustar el icono de Windows: {err}");
        }
    }
}
