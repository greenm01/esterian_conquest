fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../docs/assets/favicon.ico");
        let _ = res.compile();
    }
}
