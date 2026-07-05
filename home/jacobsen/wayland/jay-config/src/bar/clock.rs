pub fn now() -> String {
    chrono::Local::now().format("%a %d %b %H:%M").to_string()
}
