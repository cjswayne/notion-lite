mod notion;
mod render;
mod views;

use axum::{
    extract::{Path, Query, Request, State},
    http::{header, HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Router,
};
use notion::{Notion, Task};
use serde::Deserialize;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{Mutex, RwLock};

const FRESH: Duration = Duration::from_secs(60);

struct Entry {
    at: Instant,
    tasks: Arc<Vec<Task>>,
    err: Option<String>,
}

/// Stores a token that Notion has accepted; Err is shown on the setup page.
pub type SaveFn = Box<dyn Fn(&str) -> Result<(), String> + Send + Sync>;

pub enum Auth {
    None,
    /// The full expected Authorization header value.
    Basic(String),
    /// Per-launch secret: a matching `k` query parameter is swapped for a session cookie.
    Secret(String),
}

pub struct Config {
    pub token: String,
    pub auth: Auth,
    /// Desktop mode: enables /setup, which validates a token and hands it here.
    pub save_token: Option<SaveFn>,
}

struct App {
    notion: Notion,
    cache: RwLock<HashMap<String, Entry>>,
    clients: RwLock<HashMap<String, String>>,
    inflight: Mutex<()>,
    auth: Auth,
    cookie: String,
    save_token: Option<SaveFn>,
}
type S = Arc<App>;

fn now_unix() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

impl App {
    async fn warm(&self) {
        if !self.notion.has_token() {
            return;
        }
        for v in views::VIEWS.iter().filter(|v| views::canonical(v.slug) == v.slug) {
            self.refresh(v.slug, false).await;
        }
    }

    fn rejected(err: Option<&str>) -> bool {
        err.is_some_and(|e| e.starts_with("401"))
    }

    async fn refresh(&self, slug: &str, done: bool) {
        let Some(v) = views::find(slug) else { return };
        // Serialized to stay under Notion's 3 requests/second limit.
        let _g = self.inflight.lock().await;
        let filter = if done {
            views::done_filter(v, &render::unix_to_date(now_unix() - 14 * 86400))
        } else {
            views::open_filter(v)
        };
        let key = cache_key(slug, done);
        match self.notion.query(views::TASKS_DS, filter).await {
            Ok(mut tasks) => {
                if done {
                    if let Ok(open) = self.notion.query(views::TASKS_DS, views::open_filter(v)).await {
                        tasks.extend(open);
                    }
                }
                self.resolve_clients(&tasks).await;
                self.cache.write().await.insert(
                    key,
                    Entry { at: Instant::now(), tasks: Arc::new(tasks), err: None },
                );
            }
            Err(e) => {
                eprintln!("error: refresh {key}: {e}");
                let mut c = self.cache.write().await;
                let ent = c.entry(key).or_insert(Entry {
                    at: Instant::now(),
                    tasks: Arc::new(vec![]),
                    err: None,
                });
                ent.err = Some(e);
                ent.at = Instant::now();
            }
        }
    }

    async fn resolve_clients(&self, tasks: &[Task]) {
        let missing: Vec<String> = {
            let known = self.clients.read().await;
            let mut m: Vec<String> =
                tasks.iter().flat_map(|t| t.clients.iter()).filter(|id| !known.contains_key(*id)).cloned().collect();
            m.sort();
            m.dedup();
            m
        };
        for id in missing {
            if let Ok(name) = self.notion.page_title(&id).await {
                self.clients.write().await.insert(id, name);
            }
        }
    }
}

fn cache_key(slug: &str, done: bool) -> String {
    let slug = views::canonical(slug);
    if done { format!("{slug}:done") } else { slug.to_string() }
}

#[derive(Deserialize)]
struct ViewQ {
    done: Option<String>,
    refresh: Option<String>,
}

async fn view(State(app): State<S>, Path(slug): Path<String>, Query(q): Query<ViewQ>) -> Response {
    if views::find(&slug).is_none() {
        return (StatusCode::NOT_FOUND, "no such view").into_response();
    }
    let setup = app.save_token.is_some();
    if setup && !app.notion.has_token() {
        return Redirect::to("/setup").into_response();
    }
    let done = q.done.is_some();
    let key = cache_key(&slug, done);
    let fetch_slug = views::canonical(&slug).to_string();
    let state = app.cache.read().await.get(&key).map(|e| e.at.elapsed());
    match state {
        None => app.refresh(&fetch_slug, done).await,
        Some(_) if q.refresh.is_some() => app.refresh(&fetch_slug, done).await,
        Some(age) if age > FRESH => {
            // Stale-while-revalidate: serve the cached board now, refresh behind it.
            let (a, s) = (app.clone(), fetch_slug.clone());
            tokio::spawn(async move { a.refresh(&s, done).await });
        }
        _ => {}
    }
    let cache = app.cache.read().await;
    let ent = cache.get(&key);
    if setup && App::rejected(ent.and_then(|e| e.err.as_deref())) {
        return Redirect::to("/setup").into_response();
    }
    let clients = app.clients.read().await;
    let empty = Vec::new();
    let html = render::page(&render::Page {
        slug: &slug,
        tasks: ent.map(|e| e.tasks.as_slice()).unwrap_or(&empty),
        done,
        clients: &clients,
        age_secs: ent.map(|e| e.at.elapsed().as_secs()).unwrap_or(0),
        error: ent.and_then(|e| e.err.as_deref()),
        settings: setup,
    });
    ([(header::CACHE_CONTROL, "no-store")], Html(html)).into_response()
}

#[derive(Deserialize)]
struct StatusForm {
    s: String,
    back: String,
}

async fn peek(State(app): State<S>, Path(id): Path<String>) -> Response {
    let task = match app.notion.page(&id).await {
        Ok(t) => t,
        Err(e) => return (StatusCode::BAD_GATEWAY, format!("<div class=err>{}</div>", render::esc(&e))).into_response(),
    };
    app.resolve_clients(std::slice::from_ref(&task)).await;
    let mut budget = 400;
    let blocks = app.notion.blocks(&id, 0, &mut budget).await.unwrap_or_default();
    let clients = app.clients.read().await;
    Html(render::peek(&task, &clients, &blocks, now_unix())).into_response()
}

async fn set_status(
    State(app): State<S>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Form(f): Form<StatusForm>,
) -> Response {
    let from_js = headers.contains_key("x-fetch");
    let back = if f.back.starts_with("/v/") { f.back } else { "/".into() };
    if !views::STATUSES.contains(&f.s.as_str()) {
        return Redirect::to(&back).into_response();
    }
    if let Err(e) = app.notion.set_status(&id, &f.s).await {
        return (StatusCode::BAD_GATEWAY, format!("Notion refused the update: {e}")).into_response();
    }
    // Patch cached copies in place so the redirect shows the change before the next refresh.
    {
        let mut cache = app.cache.write().await;
        for (key, ent) in cache.iter_mut() {
            let done_view = key.ends_with(":done");
            let mut v: Vec<Task> = (*ent.tasks).clone();
            if let Some(t) = v.iter_mut().find(|t| t.id == id) {
                t.status = f.s.clone();
            }
            if !done_view {
                v.retain(|t| t.status != "Completed" && t.status != "Deferred");
            }
            ent.tasks = Arc::new(v);
        }
    }
    if from_js {
        return StatusCode::NO_CONTENT.into_response();
    }
    Redirect::to(&back).into_response()
}

#[derive(Deserialize)]
struct SetupForm {
    token: String,
}

fn setup_html(error: Option<&str>, has_token: bool) -> Response {
    ([(header::CACHE_CONTROL, "no-store")], Html(render::setup(error, has_token))).into_response()
}

async fn setup_page(State(app): State<S>) -> Response {
    let has = app.notion.has_token();
    let err = if has {
        app.cache
            .read()
            .await
            .values()
            .find_map(|e| e.err.as_deref().filter(|m| App::rejected(Some(m))).map(|m| format!("Notion rejected the saved token ({m}).")))
    } else {
        None
    };
    setup_html(err.as_deref(), has)
}

async fn setup_save(State(app): State<S>, Form(f): Form<SetupForm>) -> Response {
    let Some(save) = &app.save_token else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let has = app.notion.has_token();
    let token = f.token.trim().to_string();
    if token.is_empty() {
        return setup_html(Some("Paste the integration secret first."), has);
    }
    if let Err(e) = app.notion.check_token(&token).await {
        return setup_html(Some(&format!("Notion said: {e}")), has);
    }
    if let Err(e) = save(&token) {
        eprintln!("error: saving token: {e}");
        return setup_html(Some(&format!("Could not save the token: {e}")), has);
    }
    {
        // Hold the refresh lock so a request still using the old token cannot re-cache its 401.
        let _g = app.inflight.lock().await;
        app.notion.set_token(token);
        app.cache.write().await.clear();
    }
    let a = app.clone();
    tokio::spawn(async move { a.warm().await });
    Redirect::to("/v/priority").into_response()
}

async fn require_login(State(app): State<S>, req: Request, next: Next) -> Response {
    match &app.auth {
        Auth::None => {}
        Auth::Basic(expected) => {
            let ok = req
                .headers()
                .get(header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .map(|v| ct_eq(v.as_bytes(), expected.as_bytes()))
                .unwrap_or(false);
            if !ok {
                return (
                    StatusCode::UNAUTHORIZED,
                    [(header::WWW_AUTHENTICATE, "Basic realm=\"tasks\", charset=\"UTF-8\"")],
                    "login required",
                )
                    .into_response();
            }
        }
        Auth::Secret(secret) => {
            let query = req.uri().query().unwrap_or("");
            let key = query.split('&').find_map(|p| p.strip_prefix("k="));
            if key.is_some_and(|k| ct_eq(k.as_bytes(), secret.as_bytes())) {
                let rest: Vec<&str> = query.split('&').filter(|p| !p.is_empty() && !p.starts_with("k=")).collect();
                let path = req.uri().path();
                let to = if rest.is_empty() { path.to_string() } else { format!("{path}?{}", rest.join("&")) };
                return (
                    StatusCode::SEE_OTHER,
                    [
                        (header::LOCATION, to),
                        (header::SET_COOKIE, format!("{}={secret}; HttpOnly; SameSite=Strict; Path=/", app.cookie)),
                    ],
                )
                    .into_response();
            }
            let ok = req
                .headers()
                .get_all(header::COOKIE)
                .iter()
                .filter_map(|v| v.to_str().ok())
                .flat_map(|v| v.split(';'))
                .filter_map(|c| c.trim().split_once('='))
                .any(|(n, v)| n == app.cookie && ct_eq(v.as_bytes(), secret.as_bytes()));
            if !ok {
                return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
            }
        }
    }
    next.run(req).await
}

fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Serves the boards on `listener` until the server stops.
pub async fn run(listener: tokio::net::TcpListener, cfg: Config) -> std::io::Result<()> {
    let port = listener.local_addr()?.port();
    let app: S = Arc::new(App {
        notion: Notion::new(cfg.token),
        cache: RwLock::new(HashMap::new()),
        clients: RwLock::new(HashMap::new()),
        inflight: Mutex::new(()),
        auth: cfg.auth,
        // Cookies are scoped by host, not port, so each instance needs its own name.
        cookie: format!("nl_{port}"),
        save_token: cfg.save_token,
    });

    let warm = app.clone();
    tokio::spawn(async move {
        loop {
            warm.warm().await;
            tokio::time::sleep(FRESH).await;
        }
    });

    let mut routes: Router<S> = Router::new()
        .route("/", get(|| async { Redirect::to("/v/priority") }))
        .route("/v/:slug", get(view))
        .route("/t/:id", post(set_status))
        .route("/p/:id", get(peek));
    if app.save_token.is_some() {
        routes = routes.route("/setup", get(setup_page).post(setup_save));
    }
    let router = routes
        .layer(middleware::from_fn_with_state(app.clone(), require_login))
        .route("/healthz", get(|| async { "ok" }))
        .layer(tower_http::compression::CompressionLayer::new())
        .with_state(app);

    axum::serve(listener, router).await
}
