use crate::notion::Task;
use crate::views::{STATUSES, VIEWS};
use std::collections::HashMap;
use std::fmt::Write;

pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&#39;"),
            _ => o.push(c),
        }
    }
    o
}

const CSS: &str = r#":root{--bg:#f7f7f5;--col:#efefec;--card:#fff;--fg:#1f1f1d;--mut:#787774;--line:#e3e2de;--acc:#2e6fdb;
--gray:#e3e2e0;--brown:#eee0da;--orange:#fadec9;--yellow:#fdecc8;--green:#dbeddb;--blue:#d3e5ef;--purple:#e8deee;--pink:#f5e0e9;--red:#ffe2dd;color-scheme:light}
@media (prefers-color-scheme:dark){:root:not([data-theme=light]){--bg:#191919;--col:#202020;--card:#2a2a2a;--fg:#e6e6e4;--mut:#9b9a97;--line:#353535;--acc:#5b9bff;
--gray:#3a3a3a;--brown:#4a3a33;--orange:#5c3b23;--yellow:#56452a;--green:#2b4232;--blue:#28405a;--purple:#3c2f4d;--pink:#4e2c3c;--red:#5a2f2b;color-scheme:dark}}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--fg);font:14px/1.4 system-ui,-apple-system,Segoe UI,sans-serif}
header{position:sticky;top:0;z-index:2;background:var(--bg);border-bottom:1px solid var(--line);padding:8px 16px;display:flex;flex-wrap:wrap;gap:8px;align-items:center}
nav{display:flex;gap:2px;overflow-x:auto;flex:1 1 auto}nav a{padding:4px 10px;border-radius:6px;color:var(--mut);text-decoration:none;white-space:nowrap}
nav a.on{background:var(--card);color:var(--fg);box-shadow:0 0 0 1px var(--line)}nav a:hover{color:var(--fg)}
#q{flex:0 1 220px;min-width:0;padding:5px 9px;border:1px solid var(--line);border-radius:6px;background:var(--card);color:var(--fg);font:inherit}
.meta{color:var(--mut);font-size:12px;white-space:nowrap}.meta a{color:inherit}
main{display:grid;grid-auto-flow:column;grid-auto-columns:minmax(270px,1fr);gap:10px;padding:12px 16px;overflow-x:auto;align-items:start}
section{background:var(--col);border-radius:8px;padding:8px}h2{font-size:12px;font-weight:600;margin:2px 4px 8px;color:var(--mut);display:flex;justify-content:space-between}
.card{background:var(--card);border-radius:6px;padding:8px 10px;margin-bottom:6px;box-shadow:0 0 0 1px var(--line)}
.card>a{color:var(--fg);text-decoration:none;font-weight:500;display:block}.card>a:hover{color:var(--acc)}
.m{display:flex;flex-wrap:wrap;gap:4px;margin-top:6px;font-size:11.5px}.m span{padding:1px 6px;border-radius:4px;background:var(--gray)}
.cl{background:none!important;padding-left:0!important;color:var(--mut)}
.s{color:var(--mut);font-size:12.5px;margin:6px 0 0;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden}
.f{display:flex;justify-content:space-between;align-items:center;margin-top:6px;font-size:11.5px;color:var(--mut);gap:6px}
.f a{color:var(--acc)}select{font:inherit;font-size:11.5px;color:var(--mut);background:transparent;border:1px solid var(--line);border-radius:4px;padding:1px 2px;max-width:150px}
.err{margin:12px 16px;padding:10px;border-radius:6px;background:var(--red)}.empty{color:var(--mut);font-size:12px;padding:4px}
.c-gray{background:var(--gray)!important}.c-brown{background:var(--brown)!important}.c-orange{background:var(--orange)!important}.c-yellow{background:var(--yellow)!important}
.c-green{background:var(--green)!important}.c-blue{background:var(--blue)!important}.c-purple{background:var(--purple)!important}.c-pink{background:var(--pink)!important}.c-red{background:var(--red)!important}
@media (max-width:700px){main{display:block}section{margin-bottom:10px}#q{flex-basis:100%}}"#;

const JS: &str = r#"document.addEventListener('change',e=>{if(e.target.name==='s')e.target.form.submit()});
const q=document.getElementById('q');q.addEventListener('input',()=>{const v=q.value.toLowerCase();
document.querySelectorAll('.card').forEach(c=>c.hidden=v&&!c.dataset.q.includes(v))});
document.addEventListener('keydown',e=>{if(e.key==='/'&&document.activeElement!==q){e.preventDefault();q.focus()}});"#;

pub struct Page<'a> {
    pub slug: &'a str,
    pub tasks: &'a [Task],
    pub done: bool,
    pub clients: &'a HashMap<String, String>,
    pub age_secs: u64,
    pub error: Option<&'a str>,
    pub now_unix: i64,
}

pub fn page(p: &Page) -> String {
    let mut h = String::with_capacity(16 * 1024 + p.tasks.len() * 600);
    let title = VIEWS.iter().find(|v| v.slug == p.slug).map(|v| v.name).unwrap_or("Tasks");
    let _ = write!(
        h,
        "<!doctype html><html lang=en><head><meta charset=utf-8><meta name=viewport content=\"width=device-width,initial-scale=1\">\
<title>{} · Tasks</title><link rel=icon href=\"data:,\"><style>{CSS}</style></head><body><header><nav>",
        esc(title)
    );
    let done_q = if p.done { "?done=1" } else { "" };
    for v in VIEWS {
        let _ = write!(
            h,
            "<a href=\"/v/{}{done_q}\"{}>{}</a>",
            v.slug,
            if v.slug == p.slug { " class=on" } else { "" },
            v.name
        );
    }
    let _ = write!(
        h,
        "</nav><input id=q type=search placeholder=\"Filter  /\" autocomplete=off>\
<span class=meta>{} · {} · <a href=\"/v/{}?refresh=1{}\">refresh</a> · <a href=\"/v/{}{}\">{}</a></span></header>",
        p.tasks.len(),
        ago(p.age_secs as i64),
        p.slug,
        if p.done { "&done=1" } else { "" },
        p.slug,
        if p.done { "" } else { "?done=1" },
        if p.done { "hide done" } else { "show done" },
    );
    if let Some(e) = p.error {
        let _ = write!(h, "<div class=err>Notion: {}</div>", esc(e));
    }
    h.push_str("<main>");

    let mut cols: Vec<&str> = STATUSES
        .iter()
        .copied()
        .filter(|s| p.done || (*s != "Completed" && *s != "Deferred"))
        .collect();
    if p.tasks.iter().any(|t| t.status.is_empty()) {
        cols.insert(0, "");
    }
    let back = format!("/v/{}{}", p.slug, done_q);
    for col in cols {
        let items: Vec<&Task> = p.tasks.iter().filter(|t| t.status == col).collect();
        let _ = write!(
            h,
            "<section><h2><span>{}</span><span>{}</span></h2>",
            if col.is_empty() { "No status" } else { col },
            items.len()
        );
        if items.is_empty() {
            h.push_str("<div class=empty>Nothing here</div>");
        }
        for t in items {
            card(&mut h, t, p, &back);
        }
        h.push_str("</section>");
    }
    let _ = write!(h, "</main><script>{JS}</script></body></html>");
    h
}

fn card(h: &mut String, t: &Task, p: &Page, back: &str) {
    let client_names: Vec<&str> = t
        .clients
        .iter()
        .map(|id| p.clients.get(id).map(String::as_str).unwrap_or(""))
        .filter(|n| !n.is_empty())
        .collect();
    let q = format!(
        "{} {} {} {}",
        t.name,
        client_names.join(" "),
        t.tags.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(" "),
        t.summary
    )
    .to_lowercase();
    let name = if t.name.is_empty() { "Untitled" } else { &t.name };
    let _ = write!(
        h,
        "<article class=card data-q=\"{}\"><a href=\"{}\" target=_blank rel=noopener>{}</a>",
        esc(&q),
        esc(&t.url),
        esc(name)
    );
    if !client_names.is_empty() || !t.tags.is_empty() || !t.milestone.is_empty() {
        h.push_str("<div class=m>");
        for c in &client_names {
            let _ = write!(h, "<span class=cl>{}</span>", esc(c));
        }
        for tg in &t.tags {
            let _ = write!(h, "<span class=c-{}>{}</span>", esc(&tg.color), esc(&tg.name));
        }
        if !t.milestone.is_empty() {
            let _ = write!(h, "<span>M{}</span>", esc(&t.milestone));
        }
        h.push_str("</div>");
    }
    if !t.summary.is_empty() {
        let _ = write!(h, "<p class=s>{}</p>", esc(&t.summary));
    }
    let _ = write!(
        h,
        "<div class=f><form method=post action=\"/t/{}\"><input type=hidden name=back value=\"{}\"><select name=s aria-label=Status>",
        esc(&t.id),
        esc(back)
    );
    if t.status.is_empty() {
        h.push_str("<option selected>No status</option>");
    }
    for s in STATUSES {
        let _ = write!(
            h,
            "<option{}>{}</option>",
            if *s == t.status { " selected" } else { "" },
            s
        );
    }
    h.push_str("</select><noscript><button>Set</button></noscript></form><span>");
    if !t.pr.is_empty() {
        let _ = write!(h, "<a href=\"{}\" target=_blank rel=noopener>PR</a> · ", esc(&t.pr));
    }
    let _ = write!(h, "{}</span></div></article>", ago(p.now_unix - iso_to_unix(&t.edited)));
}

pub fn ago(s: i64) -> String {
    match s {
        i64::MIN..=59 => "just now".into(),
        60..=3599 => format!("{}m ago", s / 60),
        3600..=86399 => format!("{}h ago", s / 3600),
        _ => format!("{}d ago", s / 86400),
    }
}

/// Notion always sends UTC timestamps like 2026-11-24T03:32:00.000Z.
pub fn iso_to_unix(s: &str) -> i64 {
    let n = |r: std::ops::Range<usize>| s.get(r).and_then(|x| x.parse::<i64>().ok()).unwrap_or(0);
    let (y, m, d, hh, mm, ss) = (n(0..4), n(5..7), n(8..10), n(11..13), n(14..16), n(17..19));
    if y == 0 {
        return 0;
    }
    days_from_civil(y, m, d) * 86400 + hh * 3600 + mm * 60 + ss
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

pub fn unix_to_date(t: i64) -> String {
    let z = t.div_euclid(86400) + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}
