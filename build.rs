#[cfg(windows)]
fn main() {
    winres::WindowsResource::new()
        .set_icon("images/icon.ico")
        .compile()
        .unwrap();
}

#[cfg(not(windows))]
fn main() {}
