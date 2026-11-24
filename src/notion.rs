use serde_json::{json, Value};
use std::sync::{Arc, RwLock};
use std::time::Duration;

const API: &str = "https://api.notion.com/v1";
const VERSION: &str = "2025-09-03";

#[derive(Clone, Debug)]
pub struct Tag {
    pub name: String,
    pub color: String,
}

#[derive(Clone, Debug)]
pub struct Person {
    pub name: String,
    pub avatar: String,
}

#[derive(Clone, Debug)]
pub struct Task {
    pub id: String,
    pub url: String,
    pub name: String,
    pub status: String,
    pub tags: Vec<Tag>,
    pub agents: Vec<Person>,
    pub clients: Vec<String>,
    pub milestone: String,
    pub summary: String,
    pub pr: String,
    pub edited: String,
}

#[derive(Clone)]
pub struct Notion {
    http: reqwest::Client,
    token: Arc<RwLock<String>>,
}

impl Notion {
    pub fn new(token: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .pool_idle_timeout(Duration::from_secs(90))
            .build()
            .expect("http client");
        Self { http, token: Arc::new(RwLock::new(token)) }
    }

    pub fn has_token(&self) -> bool {
        !self.token.read().unwrap_or_else(|e| e.into_inner()).is_empty()
    }

    pub fn set_token(&self, token: String) {
        *self.token.write().unwrap_or_else(|e| e.into_inner()) = token;
    }

    /// Asks Notion who the given token belongs to; Err carries Notion's message.
    pub async fn check_token(&self, token: &str) -> Result<(), String> {
        self.send(self.req_as(reqwest::Method::GET, "/users/me", token)).await.map(|_| ())
    }

    fn req(&self, m: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let token = self.token.read().unwrap_or_else(|e| e.into_inner()).clone();
        self.req_as(m, path, &token)
    }

    fn req_as(&self, m: reqwest::Method, path: &str, token: &str) -> reqwest::RequestBuilder {
        self.http
            .request(m, format!("{API}{path}"))
            .bearer_auth(token)
            .header("Notion-Version", VERSION)
    }

    async fn send(&self, rb: reqwest::RequestBuilder) -> Result<Value, String> {
        for attempt in 0..2 {
            let rb2 = rb.try_clone().ok_or("unclonable request")?;
            let res = rb2.send().await.map_err(|e| e.to_string())?;
            let status = res.status();
            if (status.as_u16() == 429 || status.is_server_error()) && attempt == 0 {
                let wait = res
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(1);
                tokio::time::sleep(Duration::from_secs(wait.min(10))).await;
                continue;
            }
            let body: Value = res.json().await.map_err(|e| e.to_string())?;
            if !status.is_success() {
                return Err(format!(
                    "{}: {}",
                    status.as_u16(),
                    body["message"].as_str().unwrap_or("Notion error")
                ));
            }
            return Ok(body);
        }
        Err("retry exhausted".into())
    }

    pub async fn query(&self, ds: &str, filter: Value) -> Result<Vec<Task>, String> {
        let mut out = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let mut body = json!({
                "filter": filter,
                "page_size": 100,
                "sorts": [{"timestamp": "last_edited_time", "direction": "descending"}],
            });
            if let Some(c) = &cursor {
                body["start_cursor"] = json!(c);
            }
            let v = self
                .send(self.req(reqwest::Method::POST, &format!("/data_sources/{ds}/query")).json(&body))
                .await?;
            if let Some(rs) = v["results"].as_array() {
                out.extend(rs.iter().map(parse_task));
            }
            match (v["has_more"].as_bool(), v["next_cursor"].as_str()) {
                (Some(true), Some(c)) => cursor = Some(c.to_string()),
                _ => break,
            }
        }
        Ok(out)
    }

    pub async fn page_title(&self, id: &str) -> Result<String, String> {
        let v = self.send(self.req(reqwest::Method::GET, &format!("/pages/{id}"))).await?;
        if let Some(props) = v["properties"].as_object() {
            for p in props.values() {
                if p["type"] == "title" {
                    return Ok(plain(&p["title"]));
                }
            }
        }
        Ok(String::new())
    }

    pub async fn page(&self, id: &str) -> Result<Task, String> {
        let v = self.send(self.req(reqwest::Method::GET, &format!("/pages/{id}"))).await?;
        Ok(parse_task(&v))
    }

    /// Page body, with nested children attached under "children" (depth and size capped to stay fast).
    pub async fn blocks(&self, id: &str, depth: u8, budget: &mut usize) -> Result<Vec<Value>, String> {
        let mut out = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let mut path = format!("/blocks/{id}/children?page_size=100");
            if let Some(c) = &cursor {
                path.push_str(&format!("&start_cursor={c}"));
            }
            let v = self.send(self.req(reqwest::Method::GET, &path)).await?;
            for mut b in v["results"].as_array().cloned().unwrap_or_default() {
                if *budget == 0 {
                    return Ok(out);
                }
                *budget -= 1;
                let skip = matches!(b["type"].as_str(), Some("child_page" | "child_database"));
                if depth < 3 && !skip && b["has_children"].as_bool() == Some(true) {
                    if let Some(cid) = b["id"].as_str().map(String::from) {
                        let kids = Box::pin(self.blocks(&cid, depth + 1, budget)).await.unwrap_or_default();
                        b["children"] = Value::Array(kids);
                    }
                }
                out.push(b);
            }
            match (v["has_more"].as_bool(), v["next_cursor"].as_str()) {
                (Some(true), Some(c)) => cursor = Some(c.to_string()),
                _ => break,
            }
        }
        Ok(out)
    }

    pub async fn set_status(&self, id: &str, status: &str) -> Result<(), String> {
        let body = json!({"properties": {"Status": {"select": {"name": status}}}});
        self.send(self.req(reqwest::Method::PATCH, &format!("/pages/{id}")).json(&body))
            .await
            .map(|_| ())
    }
}

pub fn plain(rich: &Value) -> String {
    rich.as_array()
        .map(|a| a.iter().filter_map(|t| t["plain_text"].as_str()).collect())
        .unwrap_or_default()
}

fn parse_task(p: &Value) -> Task {
    let props = &p["properties"];
    let s = |k: &str| props[k]["select"]["name"].as_str().unwrap_or("").to_string();
    Task {
        id: p["id"].as_str().unwrap_or("").to_string(),
        url: p["url"].as_str().unwrap_or("").to_string(),
        name: plain(&props["Name"]["title"]),
        status: s("Status"),
        milestone: s("Milestone"),
        tags: props["Tags"]["multi_select"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|t| Tag {
                        name: t["name"].as_str().unwrap_or("").into(),
                        color: t["color"].as_str().unwrap_or("default").into(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        agents: props["Agent"]["people"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|u| Person {
                        name: u["name"].as_str().unwrap_or("").into(),
                        avatar: u["avatar_url"].as_str().unwrap_or("").into(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        clients: props["Client"]["relation"]
            .as_array()
            .map(|a| a.iter().filter_map(|r| r["id"].as_str().map(String::from)).collect())
            .unwrap_or_default(),
        summary: plain(&props["Summary"]["rich_text"]),
        pr: props["PR"]["url"].as_str().unwrap_or("").to_string(),
        edited: p["last_edited_time"].as_str().unwrap_or("").to_string(),
    }
}
