use serde_json::{json, Value};
use std::time::Duration;

const API: &str = "https://api.notion.com/v1";
const VERSION: &str = "2025-09-03";

#[derive(Clone, Debug)]
pub struct Tag {
    pub name: String,
    pub color: String,
}

#[derive(Clone, Debug)]
pub struct Task {
    pub id: String,
    pub url: String,
    pub name: String,
    pub status: String,
    pub tags: Vec<Tag>,
    pub clients: Vec<String>,
    pub milestone: String,
    pub summary: String,
    pub pr: String,
    pub edited: String,
}

#[derive(Clone)]
pub struct Notion {
    http: reqwest::Client,
    token: String,
}

impl Notion {
    pub fn new(token: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .pool_idle_timeout(Duration::from_secs(90))
            .build()
            .expect("http client");
        Self { http, token }
    }

    fn req(&self, m: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.http
            .request(m, format!("{API}{path}"))
            .bearer_auth(&self.token)
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

    pub async fn set_status(&self, id: &str, status: &str) -> Result<(), String> {
        let body = json!({"properties": {"Status": {"select": {"name": status}}}});
        self.send(self.req(reqwest::Method::PATCH, &format!("/pages/{id}")).json(&body))
            .await
            .map(|_| ())
    }
}

fn plain(rich: &Value) -> String {
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
        clients: props["Client"]["relation"]
            .as_array()
            .map(|a| a.iter().filter_map(|r| r["id"].as_str().map(String::from)).collect())
            .unwrap_or_default(),
        summary: plain(&props["Summary"]["rich_text"]),
        pr: props["PR"]["url"].as_str().unwrap_or("").to_string(),
        edited: p["last_edited_time"].as_str().unwrap_or("").to_string(),
    }
}
