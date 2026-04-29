fn main() {
    if cfg!(target_os = "windows") && std::env::var_os("CARGO_PRIMARY_PACKAGE").is_some() {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.compile().unwrap();
    }
}
