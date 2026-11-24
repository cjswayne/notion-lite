#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use notion_lite::{Auth, Config, SaveFn};
use tauri::webview::NewWindowResponse;
use tauri::{AppHandle, Url, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

const SERVICE: &str = "notion-lite";
const ACCOUNT: &str = "notion-token";

// Some WebView2 builds never report new-tab link clicks to on_new_window, so turn them into navigations.
const LINKS_OUT: &str = r#"(()=>{const ext=u=>{try{const x=new URL(u,location.href);return x.origin!==location.origin?x.href:null}catch(e){return null}};
document.addEventListener('click',e=>{const a=e.target.closest&&e.target.closest('a[href]');if(!a)return;const u=ext(a.href);if(u){e.preventDefault();location.href=u}},true);
const o=window.open;window.open=function(u,...r){const x=ext(u);if(x){location.href=x;return null}return o.call(window,u,...r)}})();"#;

fn load_token() -> String {
    match keyring::Entry::new(SERVICE, ACCOUNT).and_then(|e| e.get_password()) {
        Ok(t) => t,
        Err(keyring::Error::NoEntry) => String::new(),
        Err(e) => {
            eprintln!("warning: reading the Notion token from Credential Manager: {e}");
            String::new()
        }
    }
}

fn save_token(token: &str) -> Result<(), String> {
    keyring::Entry::new(SERVICE, ACCOUNT)
        .and_then(|e| e.set_password(token))
        .map_err(|e| format!("Windows Credential Manager: {e}"))
}

fn secret() -> Result<String, String> {
    let mut b = [0u8; 32];
    getrandom::fill(&mut b).map_err(|e| format!("random secret: {e}"))?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

fn is_local(url: &Url, port: u16) -> bool {
    url.scheme() == "http" && url.host_str() == Some("127.0.0.1") && url.port() == Some(port)
}

fn open_external(app: &AppHandle, url: &Url) {
    if !matches!(url.scheme(), "http" | "https" | "mailto" | "tel") {
        return;
    }
    if let Err(e) = app.opener().open_url(url.as_str(), None::<&str>) {
        eprintln!("error: opening {url}: {e}");
    }
}

fn main() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let listener = std::net::TcpListener::bind(("127.0.0.1", 0))?;
            listener.set_nonblocking(true)?;
            let port = listener.local_addr()?.port();
            let secret = secret()?;
            let cfg = Config {
                token: load_token(),
                auth: Auth::Secret(secret.clone()),
                save_token: Some(Box::new(save_token) as SaveFn),
            };
            tauri::async_runtime::spawn(async move {
                let served = match tokio::net::TcpListener::from_std(listener) {
                    Ok(l) => notion_lite::run(l, cfg).await,
                    Err(e) => Err(e),
                };
                if let Err(e) = served {
                    eprintln!("error: local server: {e}");
                }
            });

            let url: Url = format!("http://127.0.0.1:{port}/?k={secret}").parse()?;
            let (nav, popup) = (app.handle().clone(), app.handle().clone());
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url))
                .title("Notion Lite")
                .inner_size(1400.0, 900.0)
                .initialization_script(LINKS_OUT)
                .on_navigation(move |url| {
                    if is_local(url, port) {
                        return true;
                    }
                    open_external(&nav, url);
                    url.scheme() == "about"
                })
                .on_new_window(move |url, _features| {
                    open_external(&popup, &url);
                    NewWindowResponse::Deny
                })
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!());
    if let Err(e) = result {
        eprintln!("error: {e}");
    }
}
