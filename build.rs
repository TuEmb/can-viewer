#[cfg(target_os = "windows")]
use winresource::WindowsResource;

fn main() {
    slint_build::compile("ui/app.slint").unwrap();
    #[cfg(target_os = "windows")]
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        WindowsResource::new()
            // This path can be absolute, or relative to your crate root.
            .set_icon("ui/images/can_viewer.ico")
            .compile()
            .unwrap();
    }
}
