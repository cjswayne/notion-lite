use base64::Engine;
use notion_lite::{Auth, Config};

#[tokio::main]
async fn main() {
    let token = std::env::var("NOTION_TOKEN").expect("set NOTION_TOKEN");
    let auth = match (std::env::var("DASH_USER"), std::env::var("DASH_PASSWORD")) {
        (Ok(u), Ok(p)) if !p.is_empty() => Auth::Basic(format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(format!("{u}:{p}"))
        )),
        _ => {
            eprintln!("warning: DASH_USER/DASH_PASSWORD not set, no login required");
            Auth::None
        }
    };
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}")).await.unwrap();
    notion_lite::run(listener, Config { token, auth, save_token: None }).await.unwrap();
}
