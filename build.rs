fn main() {
    if std::env::var("PROFILE").unwrap_or_default() == "release" {
        #[cfg(target_os = "windows")]
        {
            println!("cargo:rustc-link-arg=/SUBSYSTEM:WINDOWS");
            println!("cargo:rustc-link-arg=/ENTRY:mainCRTStartup");
        }
    }
}
