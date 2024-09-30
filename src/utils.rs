pub fn http_to_ws(url: &str) -> String {
    url.replace("http://", "ws://").replace("https://", "wss://")
}
