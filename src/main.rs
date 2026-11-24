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

struct App {
    notion: Notion,
    cache: RwLock<HashMap<String, Entry>>,
    clients: RwLock<HashMap<String, String>>,
    inflight: Mutex<()>,
    auth: Option<String>,
}
type S = Arc<App>;

fn now_unix() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

impl App {
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
    let clients = app.clients.read().await;
    let ent = cache.get(&key);
    let empty = Vec::new();
    let html = render::page(&render::Page {
        slug: &slug,
        tasks: ent.map(|e| e.tasks.as_slice()).unwrap_or(&empty),
        done,
        clients: &clients,
        age_secs: ent.map(|e| e.at.elapsed().as_secs()).unwrap_or(0),
        error: ent.and_then(|e| e.err.as_deref()),
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

async fn require_login(State(app): State<S>, headers: HeaderMap, req: Request, next: Next) -> Response {
    if let Some(expected) = &app.auth {
        let ok = headers
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
    next.run(req).await
}

fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[tokio::main]
async fn main() {
    use base64::Engine;
    let token = std::env::var("NOTION_TOKEN").expect("set NOTION_TOKEN");
    let auth = match (std::env::var("DASH_USER"), std::env::var("DASH_PASSWORD")) {
        (Ok(u), Ok(p)) if !p.is_empty() => Some(format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(format!("{u}:{p}"))
        )),
        _ => {
            eprintln!("warning: DASH_USER/DASH_PASSWORD not set, no login required");
            None
        }
    };
    let app: S = Arc::new(App {
        notion: Notion::new(token),
        cache: RwLock::new(HashMap::new()),
        clients: RwLock::new(HashMap::new()),
        inflight: Mutex::new(()),
        auth,
    });

    let warm = app.clone();
    tokio::spawn(async move {
        loop {
            for v in views::VIEWS.iter().filter(|v| views::canonical(v.slug) == v.slug) {
                warm.refresh(v.slug, false).await;
            }
            tokio::time::sleep(FRESH).await;
        }
    });

    let router = Router::new()
        .route("/", get(|| async { Redirect::to("/v/priority") }))
        .route("/v/:slug", get(view))
        .route("/t/:id", post(set_status))
        .route("/p/:id", get(peek))
        .layer(middleware::from_fn_with_state(app.clone(), require_login))
        .route("/healthz", get(|| async { "ok" }))
        .layer(tower_http::compression::CompressionLayer::new())
        .with_state(app);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
    axum::serve(listener, router).await.unwrap();
}
