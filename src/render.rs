use crate::notion::{plain, Task};
use crate::views::{STATUSES, VIEWS};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
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

pub fn status_color(s: &str) -> &'static str {
    match s {
        "Up Next" => "gray",
        "In Progress - JadePuma" => "green",
        "In Progress / To Do - Client" => "orange",
        "Completed" => "pink",
        "Deferred" => "blue",
        _ => "default",
    }
}

fn milestone_color(m: &str) -> &'static str {
    match m {
        "1" => "brown",
        "2" => "green",
        "3" => "blue",
        "4" => "yellow",
        "6" => "pink",
        _ => "default",
    }
}

const CSS: &str = r#":root{--bg:#fff;--fg:rgb(55,53,47);--mut:rgba(55,53,47,.5);--line:rgba(55,53,47,.09);--card:#fff;--hover:rgba(55,53,47,.06);
--sh:rgba(15,15,15,.1) 0 0 0 1px,rgba(15,15,15,.1) 0 2px 4px;--peek:#fff;
--t-default:rgba(206,205,202,.5);--t-gray:rgb(227,226,224);--t-brown:rgb(238,224,218);--t-orange:rgb(250,222,201);--t-yellow:rgb(253,236,200);--t-green:rgb(219,237,219);--t-blue:rgb(211,229,239);--t-purple:rgb(232,222,238);--t-pink:rgb(245,224,233);--t-red:rgb(255,226,221);
--g-default:rgba(247,247,245,.7);--g-gray:rgba(248,248,247,1);--g-brown:rgba(250,246,245,1);--g-orange:rgba(252,245,242,1);--g-yellow:rgba(250,247,237,1);--g-green:rgba(244,248,243,1);--g-blue:rgba(241,248,251,1);--g-purple:rgba(249,246,252,1);--g-pink:rgba(251,245,251,1);--g-red:rgba(253,245,243,1);
--d-default:rgba(55,53,47,.35);--d-gray:rgb(145,145,142);--d-brown:rgb(187,132,108);--d-orange:rgb(215,129,58);--d-yellow:rgb(203,148,52);--d-green:rgb(108,155,125);--d-blue:rgb(91,151,189);--d-purple:rgb(167,130,195);--d-pink:rgb(205,116,159);--d-red:rgb(225,111,100);
--tf:rgb(50,48,44);color-scheme:light}
@media (prefers-color-scheme:dark){:root:not([data-theme=light]){--bg:#191919;--fg:rgba(255,255,255,.81);--mut:rgba(255,255,255,.46);--line:rgba(255,255,255,.094);--card:#252525;--hover:rgba(255,255,255,.055);
--sh:rgba(15,15,15,.1) 0 0 0 1px,rgba(15,15,15,.2) 0 2px 4px;--peek:#202020;
--t-default:rgba(255,255,255,.094);--t-gray:rgb(71,76,80);--t-brown:rgb(92,71,61);--t-orange:rgb(136,84,44);--t-yellow:rgb(146,118,63);--t-green:rgb(36,61,48);--t-blue:rgb(20,58,78);--t-purple:rgb(60,45,73);--t-pink:rgb(78,44,60);--t-red:rgb(82,46,42);
--g-default:rgba(255,255,255,.02);--g-gray:rgba(28,28,28,1);--g-brown:rgba(30,26,24,1);--g-orange:rgba(33,27,22,1);--g-yellow:rgba(32,29,22,1);--g-green:rgba(23,29,25,1);--g-blue:rgba(21,27,31,1);--g-purple:rgba(28,24,31,1);--g-pink:rgba(31,24,27,1);--g-red:rgba(33,24,23,1);
--tf:rgba(255,255,255,.81);color-scheme:dark}}
*{box-sizing:border-box}html,body{margin:0;background:var(--bg);color:var(--fg)}
body{font:14px/1.5 ui-sans-serif,-apple-system,BlinkMacSystemFont,"Segoe UI Variable Display","Segoe UI",Helvetica,"Apple Color Emoji",Arial,sans-serif;-webkit-font-smoothing:antialiased}
a{color:inherit}.top{height:45px;display:flex;align-items:center;padding:0 12px;font-size:14px;gap:6px;color:var(--fg)}
.top .crumb{padding:2px 6px;border-radius:4px}.top .sp{flex:1}.top input{width:200px;max-width:40vw;padding:4px 8px;border-radius:6px;border:none;background:var(--hover);color:var(--fg);font:inherit;font-size:13px;outline:none}
.wrap{padding:0 96px 40px}h1{font-size:40px;font-weight:700;line-height:1.2;margin:40px 0 18px;letter-spacing:-.01em}
.tabs{display:flex;align-items:center;gap:2px;border-bottom:1px solid var(--line);overflow-x:auto;scrollbar-width:none}.tabs::-webkit-scrollbar{display:none}
.tabs a{flex:none;padding:6px 8px 7px;font-size:14px;color:var(--mut);text-decoration:none;border-radius:6px 6px 0 0;position:relative;white-space:nowrap;font-weight:500}
.tabs a:hover{background:var(--hover)}.tabs a.on{color:var(--fg)}.tabs a.on:after{content:"";position:absolute;left:0;right:0;bottom:-1px;height:2px;background:var(--fg)}
.bar{display:flex;justify-content:flex-end;gap:12px;font-size:12px;color:var(--mut);padding:6px 0 10px}.bar a{text-decoration:none}.bar a:hover{color:var(--fg)}
.board{display:flex;gap:12px;overflow-x:auto;align-items:flex-start;padding-bottom:12px}
.col{flex:none;width:260px;border-radius:10px;padding:0 6px 6px;background:var(--g-default)}
.colh{display:flex;align-items:center;gap:6px;height:42px;padding:0 4px;font-size:14px}.colh .n{color:var(--mut)}
.pill{display:inline-flex;align-items:center;gap:6px;height:20px;padding:0 8px;border-radius:10px;font-size:14px;line-height:120%;color:var(--tf);white-space:nowrap;max-width:100%;overflow:hidden;text-overflow:ellipsis}
.pill.sq{border-radius:3px;padding:0 6px;font-size:12px;height:18px}.dot{width:8px;height:8px;border-radius:50%;flex:none}
.cards{min-height:30px;display:flex;flex-direction:column;gap:8px;border-radius:8px}.cards.over{outline:2px solid rgba(35,131,226,.5)}
.card{background:var(--card);border-radius:10px;box-shadow:var(--sh);padding:10px;cursor:pointer;user-select:none}.card:hover{background:color-mix(in srgb,var(--card),var(--fg) 3%)}
.card.drag{opacity:.4}.ct{font-weight:500;font-size:14px;line-height:1.5;word-break:break-word}
.props{display:flex;flex-direction:column;gap:4px;margin-top:6px}.p{display:flex;flex-wrap:wrap;gap:4px;align-items:center;font-size:12px;min-height:20px}
.av{width:20px;height:20px;border-radius:50%;object-fit:cover;flex:none}.ini{display:inline-flex;align-items:center;justify-content:center;background:var(--t-gray);font-size:11px}
.person{display:inline-flex;align-items:center;gap:6px;font-size:12px;margin-right:6px}
.rel{display:inline-flex;align-items:center;gap:3px;font-size:12px;border-bottom:1px solid var(--line);line-height:1.3}.rel svg{width:14px;height:14px;opacity:.6;flex:none}
.hid{flex:none;width:220px;font-size:14px;color:var(--mut)}.hid .colh{color:var(--mut)}.hid a{display:flex;justify-content:space-between;padding:6px 8px;border-radius:6px;text-decoration:none}.hid a:hover{background:var(--hover)}
.lane{margin:6px 0 14px}.lane>summary{list-style:none;cursor:pointer;display:flex;align-items:center;gap:6px;padding:6px 2px;font-weight:500}
.lane>summary::-webkit-details-marker{display:none}.lane>summary:before{content:"";width:0;height:0;border:4px solid transparent;border-left:6px solid var(--mut);margin-right:2px;transition:transform .1s}
.lane[open]>summary:before{transform:rotate(90deg) translateX(2px)}.lane .n{color:var(--mut);font-weight:400}
.err{margin:10px 0;padding:10px 12px;border-radius:6px;background:var(--t-red)}.empty{color:var(--mut);font-size:13px;padding:4px}
#peek{position:fixed;top:0;right:0;bottom:0;width:min(640px,100vw);background:var(--peek);box-shadow:rgba(15,15,15,.04) 0 0 0 1px,rgba(15,15,15,.03) 0 3px 6px,rgba(15,15,15,.06) 0 9px 24px;z-index:9;display:flex;flex-direction:column}
#peek[hidden]{display:none}.pkbar{display:flex;align-items:center;gap:8px;height:44px;padding:0 12px;flex:none}.pkbar button,.pkbar a{background:none;border:0;color:var(--mut);font:inherit;font-size:14px;cursor:pointer;padding:4px 8px;border-radius:6px;text-decoration:none}
.pkbar button:hover,.pkbar a:hover{background:var(--hover)}.pkbar .sp{flex:1}#pkb{overflow-y:auto;padding:0 48px 60px;flex:1}
#pkb h1{font-size:32px;margin:24px 0 14px}.pt{width:100%;border-collapse:collapse;margin-bottom:12px}.pt td{padding:5px 6px;vertical-align:top;font-size:14px}
.pt td:first-child{width:160px;color:var(--mut)}.pt select{font:inherit;background:var(--hover);color:var(--fg);border:0;border-radius:4px;padding:2px 4px}
.doc{border-top:1px solid var(--line);padding-top:10px;line-height:1.5}.doc p{margin:4px 0;min-height:1.5em}.doc h2{font-size:1.875em;margin:1.4em 0 .2em}.doc h3{font-size:1.5em;margin:1.2em 0 .2em}.doc h4{font-size:1.25em;margin:1em 0 .2em}
.doc ul,.doc ol{margin:2px 0;padding-left:1.6em}.doc li{padding:2px 0}.doc blockquote{margin:6px 0;padding:0 14px;border-left:3px solid currentColor}
.doc .callout{display:flex;gap:8px;padding:14px 14px 14px 12px;border-radius:6px;background:var(--hover);margin:6px 0}.doc pre{background:var(--hover);padding:16px;border-radius:6px;overflow:auto;font-size:13px}
.doc code{font-family:SFMono-Regular,Menlo,Consolas,monospace;background:var(--hover);color:#eb5757;border-radius:4px;padding:.2em .4em;font-size:85%}.doc pre code{background:none;color:inherit;padding:0}
.doc hr{border:0;border-top:1px solid var(--line);margin:12px 0}.doc img{max-width:100%;border-radius:4px}.doc .todo{display:flex;gap:8px;align-items:flex-start;margin:2px 0}.doc .todo.done span{text-decoration:line-through;opacity:.6}
.doc details{margin:2px 0}.doc details>div{padding-left:1.4em}.doc table{border-collapse:collapse;margin:8px 0}.doc td{border:1px solid var(--line);padding:6px 8px;font-size:14px}
.doc .bm{display:block;border:1px solid var(--line);border-radius:6px;padding:10px 12px;margin:6px 0;font-size:13px;text-decoration:none;word-break:break-all}.loading{color:var(--mut);padding:24px 0}
.tc-gray{color:rgb(120,119,116)}.tc-brown{color:rgb(159,107,83)}.tc-orange{color:rgb(217,115,13)}.tc-yellow{color:rgb(203,145,47)}.tc-green{color:rgb(68,131,97)}.tc-blue{color:rgb(51,126,169)}.tc-purple{color:rgb(144,101,176)}.tc-pink{color:rgb(193,76,138)}.tc-red{color:rgb(212,76,71)}
@media (max-width:800px){.wrap{padding:0 16px 40px}h1{font-size:30px;margin-top:20px}#pkb{padding:0 20px 40px}}"#;

const JS: &str = r#"const $=s=>document.querySelector(s),$$=s=>document.querySelectorAll(s);
const q=$('#q');q.addEventListener('input',()=>{const v=q.value.toLowerCase();$$('.card').forEach(c=>c.hidden=v&&!c.dataset.q.includes(v))});
const peek=$('#peek'),pkb=$('#pkb'),pkn=$('#pkn');
function open(c){peek.hidden=false;pkn.href=c.dataset.url;pkb.innerHTML='<div class=loading>Loading…</div>';
fetch('/p/'+c.dataset.id).then(r=>r.text()).then(h=>{pkb.innerHTML=h;pkb.scrollTop=0}).catch(()=>pkb.innerHTML='<div class=err>Could not load page</div>')}
function shut(){peek.hidden=true}
$('#pkx').onclick=shut;document.addEventListener('keydown',e=>{if(e.key==='Escape')shut();if(e.key==='/'&&document.activeElement!==q){e.preventDefault();q.focus()}});
document.addEventListener('click',e=>{const c=e.target.closest('.card');if(!c)return;if(e.metaKey||e.ctrlKey){window.open(c.dataset.url,'_blank');return}open(c)});
function setStatus(id,s){return fetch('/t/'+id,{method:'POST',headers:{'x-fetch':'1','content-type':'application/x-www-form-urlencoded'},body:'s='+encodeURIComponent(s)+'&back=/'}).then(r=>{if(!r.ok)throw 0})}
pkb.addEventListener('change',e=>{if(e.target.name==='s')setStatus(e.target.dataset.id,e.target.value).then(()=>location.reload()).catch(()=>alert('Notion refused the update'))});
function counts(){$$('.col[data-status]').forEach(c=>c.querySelector('.n').textContent=c.querySelectorAll('.card').length)}
let dragged=null;
document.addEventListener('dragstart',e=>{const c=e.target.closest('.card');if(!c)return;dragged=c;c.classList.add('drag');e.dataTransfer.effectAllowed='move'});
document.addEventListener('dragend',()=>{if(dragged)dragged.classList.remove('drag');dragged=null;$$('.over').forEach(x=>x.classList.remove('over'))});
$$('.cards').forEach(z=>{z.addEventListener('dragover',e=>{if(!dragged)return;e.preventDefault();z.classList.add('over');
const after=[...z.querySelectorAll('.card:not(.drag)')].find(c=>{const r=c.getBoundingClientRect();return e.clientY<r.top+r.height/2});
after?z.insertBefore(dragged,after):z.appendChild(dragged)});
z.addEventListener('dragleave',()=>z.classList.remove('over'));
z.addEventListener('drop',e=>{e.preventDefault();z.classList.remove('over');const c=dragged;if(!c)return;const s=z.closest('.col').dataset.status;
if(c.dataset.status!==s){const prev=c.dataset.status;c.dataset.status=s;counts();setStatus(c.dataset.id,s).catch(()=>{alert('Notion refused the update');location.reload()})}})});"#;

const DOC_ICON: &str = r#"<svg viewBox="0 0 16 16" fill="currentColor"><path d="M4.4 1.5h4.9l3.2 3.2v8.3c0 .8-.7 1.5-1.5 1.5H4.4c-.8 0-1.5-.7-1.5-1.5V3c0-.8.7-1.5 1.5-1.5zm0 1.2c-.2 0-.3.1-.3.3v10c0 .2.1.3.3.3h6.6c.2 0 .3-.1.3-.3V5.5H9.2c-.5 0-.9-.4-.9-.9V2.7H4.4z"/></svg>"#;

pub struct Page<'a> {
    pub slug: &'a str,
    pub tasks: &'a [Task],
    pub done: bool,
    pub clients: &'a HashMap<String, String>,
    pub age_secs: u64,
    pub error: Option<&'a str>,
}

pub fn page(p: &Page) -> String {
    let mut h = String::with_capacity(32 * 1024 + p.tasks.len() * 900);
    let view = VIEWS.iter().find(|v| v.slug == p.slug);
    let title = view.map(|v| v.name).unwrap_or("Tasks");
    let _ = write!(
        h,
        "<!doctype html><html lang=en><head><meta charset=utf-8><meta name=viewport content=\"width=device-width,initial-scale=1\">\
<title>{} · View of Tasks</title><link rel=icon href=\"data:,\"><style>{CSS}</style></head><body>\
<div class=top><span class=crumb>Charles</span><span class=sp></span><input id=q type=search placeholder=\"Search  /\" autocomplete=off></div>\
<div class=wrap><h1>View of Tasks</h1><div class=tabs>",
        esc(title)
    );
    let done_q = if p.done { "?done=1" } else { "" };
    for v in VIEWS {
        let _ = write!(
            h,
            "<a href=\"/v/{}{done_q}\"{}>{}</a>",
            v.slug,
            if v.slug == p.slug { " class=on" } else { "" },
            esc(v.name)
        );
    }
    let _ = write!(
        h,
        "</div><div class=bar><span>{} tasks · updated {}</span><a href=\"/v/{}?refresh=1{}\">Refresh</a><a href=\"/v/{}{}\">{}</a></div>",
        p.tasks.len(),
        ago(p.age_secs as i64),
        p.slug,
        if p.done { "&done=1" } else { "" },
        p.slug,
        if p.done { "" } else { "?done=1" },
        if p.done { "Hide completed" } else { "Show completed" },
    );
    if let Some(e) = p.error {
        let _ = write!(h, "<div class=err>Notion: {}</div>", esc(e));
    }

    let cols: Vec<&str> = STATUSES
        .iter()
        .copied()
        .filter(|s| p.done || (*s != "Completed" && *s != "Deferred"))
        .collect();

    if view.map(|v| v.sub_group_client).unwrap_or(false) {
        let mut lanes: BTreeMap<String, Vec<&Task>> = BTreeMap::new();
        let mut none: Vec<&Task> = Vec::new();
        for t in p.tasks {
            let names: Vec<&String> = t.clients.iter().filter_map(|id| p.clients.get(id)).filter(|n| !n.is_empty()).collect();
            if names.is_empty() {
                none.push(t);
            }
            for n in names {
                lanes.entry(n.clone()).or_default().push(t);
            }
        }
        let mut all: Vec<(String, Vec<&Task>)> = lanes.into_iter().collect();
        if !none.is_empty() {
            all.push(("No Client".into(), none));
        }
        for (name, tasks) in all {
            let _ = write!(
                h,
                "<details class=lane open><summary>{}<span class=n>{}</span></summary>",
                esc(&name),
                tasks.len()
            );
            board(&mut h, &cols, &tasks, p);
            h.push_str("</details>");
        }
    } else {
        let tasks: Vec<&Task> = p.tasks.iter().collect();
        board(&mut h, &cols, &tasks, p);
    }

    let _ = write!(
        h,
        "</div><aside id=peek hidden><div class=pkbar><button id=pkx title=Close>✕</button><span class=sp></span>\
<a id=pkn target=_blank rel=noopener>Open in Notion ↗</a></div><div id=pkb></div></aside><script>{JS}</script></body></html>"
    );
    h
}

fn board(h: &mut String, cols: &[&str], tasks: &[&Task], p: &Page) {
    h.push_str("<div class=board>");
    if tasks.iter().any(|t| t.status.is_empty()) {
        column(h, "", tasks, p);
    }
    for col in cols {
        column(h, col, tasks, p);
    }
    if !p.done {
        let _ = write!(
            h,
            "<div class=hid><div class=colh>Hidden groups</div><a href=\"/v/{0}?done=1\"><span>Completed</span><span>›</span></a>\
<a href=\"/v/{0}?done=1\"><span>Deferred</span><span>›</span></a></div>",
            p.slug
        );
    }
    h.push_str("</div>");
}

fn column(h: &mut String, col: &str, tasks: &[&Task], p: &Page) {
    let items: Vec<&&Task> = tasks.iter().filter(|t| t.status == col).collect();
    let color = status_color(col);
    let label = if col.is_empty() { "No Status" } else { col };
    let _ = write!(
        h,
        "<section class=col data-status=\"{}\" style=\"background:var(--g-{color})\"><div class=colh>\
<span class=pill style=\"background:var(--t-{color})\"><i class=dot style=\"background:var(--d-{color})\"></i>{}</span><span class=n>{}</span></div><div class=cards>",
        esc(col),
        esc(label),
        items.len()
    );
    for t in items {
        card(h, t, p);
    }
    h.push_str("</div></section>");
}

fn initials(name: &str) -> String {
    name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default()
}

fn person(h: &mut String, name: &str, avatar: &str) {
    if avatar.is_empty() {
        let _ = write!(h, "<span class=person><span class=\"av ini\">{}</span>{}</span>", esc(&initials(name)), esc(name));
    } else {
        let _ = write!(
            h,
            "<span class=person><img class=av src=\"{}\" alt=\"\" loading=lazy referrerpolicy=no-referrer>{}</span>",
            esc(avatar),
            esc(name)
        );
    }
}

fn client_names<'a>(t: &Task, clients: &'a HashMap<String, String>) -> Vec<&'a str> {
    t.clients
        .iter()
        .filter_map(|id| clients.get(id).map(String::as_str))
        .filter(|n| !n.is_empty())
        .collect()
}

fn card(h: &mut String, t: &Task, p: &Page) {
    let cn = client_names(t, p.clients);
    let q = format!(
        "{} {} {} {}",
        t.name,
        cn.join(" "),
        t.tags.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(" "),
        t.agents.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(" ")
    )
    .to_lowercase();
    let _ = write!(
        h,
        "<article class=card draggable=true data-id=\"{}\" data-url=\"{}\" data-status=\"{}\" data-q=\"{}\"><div class=ct>{}</div><div class=props>",
        esc(&t.id),
        esc(&t.url),
        esc(&t.status),
        esc(&q),
        esc(if t.name.is_empty() { "Untitled" } else { &t.name })
    );
    if !t.agents.is_empty() {
        h.push_str("<div class=p>");
        for a in &t.agents {
            person(h, &a.name, &a.avatar);
        }
        h.push_str("</div>");
    }
    if !cn.is_empty() {
        h.push_str("<div class=p>");
        for c in &cn {
            let _ = write!(h, "<span class=rel>{DOC_ICON}{}</span>", esc(c));
        }
        h.push_str("</div>");
    }
    if !t.tags.is_empty() {
        h.push_str("<div class=p>");
        for tg in &t.tags {
            let _ = write!(h, "<span class=\"pill sq\" style=\"background:var(--t-{})\">{}</span>", css_color(&tg.color), esc(&tg.name));
        }
        h.push_str("</div>");
    }
    if !t.milestone.is_empty() {
        let _ = write!(
            h,
            "<div class=p><span class=\"pill sq\" style=\"background:var(--t-{})\">{}</span></div>",
            milestone_color(&t.milestone),
            esc(&t.milestone)
        );
    }
    h.push_str("</div></article>");
}

fn css_color(c: &str) -> &'static str {
    match c {
        "gray" => "gray",
        "brown" => "brown",
        "orange" => "orange",
        "yellow" => "yellow",
        "green" => "green",
        "blue" => "blue",
        "purple" => "purple",
        "pink" => "pink",
        "red" => "red",
        _ => "default",
    }
}

/// Side-peek fragment: title, properties and page body.
pub fn peek(t: &Task, clients: &HashMap<String, String>, blocks: &[Value], now_unix: i64) -> String {
    let mut h = String::with_capacity(8 * 1024);
    let _ = write!(h, "<h1>{}</h1><table class=pt>", esc(if t.name.is_empty() { "Untitled" } else { &t.name }));
    let _ = write!(h, "<tr><td>Status</td><td><select name=s data-id=\"{}\">", esc(&t.id));
    if t.status.is_empty() {
        h.push_str("<option selected>No Status</option>");
    }
    for s in STATUSES {
        let _ = write!(h, "<option{}>{}</option>", if *s == t.status { " selected" } else { "" }, s);
    }
    h.push_str("</select></td></tr><tr><td>Agent</td><td>");
    for a in &t.agents {
        person(&mut h, &a.name, &a.avatar);
    }
    h.push_str("</td></tr><tr><td>Client</td><td>");
    for c in client_names(t, clients) {
        let _ = write!(h, "<span class=rel>{DOC_ICON}{}</span> ", esc(c));
    }
    h.push_str("</td></tr><tr><td>Tags</td><td>");
    for tg in &t.tags {
        let _ = write!(h, "<span class=\"pill sq\" style=\"background:var(--t-{})\">{}</span> ", css_color(&tg.color), esc(&tg.name));
    }
    h.push_str("</td></tr>");
    if !t.milestone.is_empty() {
        let _ = write!(
            h,
            "<tr><td>Milestone</td><td><span class=\"pill sq\" style=\"background:var(--t-{})\">{}</span></td></tr>",
            milestone_color(&t.milestone),
            esc(&t.milestone)
        );
    }
    if !t.pr.is_empty() {
        let _ = write!(h, "<tr><td>PR</td><td><a href=\"{0}\" target=_blank rel=noopener>{0}</a></td></tr>", esc(&t.pr));
    }
    if !t.summary.is_empty() {
        let _ = write!(h, "<tr><td>Summary</td><td>{}</td></tr>", esc(&t.summary));
    }
    let _ = write!(h, "<tr><td>Last edited</td><td>{}</td></tr></table><div class=doc>", ago(now_unix - iso_to_unix(&t.edited)));
    render_blocks(&mut h, blocks);
    h.push_str("</div>");
    h
}

fn rich(v: &Value) -> String {
    let mut o = String::new();
    for r in v.as_array().into_iter().flatten() {
        let mut s = esc(r["plain_text"].as_str().unwrap_or("")).replace('\n', "<br>");
        let a = &r["annotations"];
        if a["code"] == true {
            s = format!("<code>{s}</code>");
        }
        if a["bold"] == true {
            s = format!("<b>{s}</b>");
        }
        if a["italic"] == true {
            s = format!("<i>{s}</i>");
        }
        if a["strikethrough"] == true {
            s = format!("<s>{s}</s>");
        }
        if a["underline"] == true {
            s = format!("<u>{s}</u>");
        }
        if let Some(c) = a["color"].as_str() {
            let c = c.trim_end_matches("_background");
            if c != "default" {
                s = format!("<span class=tc-{}>{s}</span>", css_color(c));
            }
        }
        if let Some(href) = r["href"].as_str() {
            s = format!("<a href=\"{}\" target=_blank rel=noopener>{s}</a>", esc(href));
        }
        o.push_str(&s);
    }
    o
}

fn file_url(b: &Value) -> &str {
    b["file"]["url"].as_str().or(b["external"]["url"].as_str()).unwrap_or("")
}

fn render_blocks(h: &mut String, blocks: &[Value]) {
    let mut i = 0;
    while i < blocks.len() {
        let b = &blocks[i];
        let ty = b["type"].as_str().unwrap_or("");
        let d = &b[ty];
        let kids = b["children"].as_array().map(Vec::as_slice).unwrap_or(&[]);
        match ty {
            "bulleted_list_item" | "numbered_list_item" => {
                let tag = if ty == "bulleted_list_item" { "ul" } else { "ol" };
                let _ = write!(h, "<{tag}>");
                while i < blocks.len() && blocks[i]["type"] == ty {
                    let li = &blocks[i];
                    let _ = write!(h, "<li>{}", rich(&li[ty]["rich_text"]));
                    render_blocks(h, li["children"].as_array().map(Vec::as_slice).unwrap_or(&[]));
                    h.push_str("</li>");
                    i += 1;
                }
                let _ = write!(h, "</{tag}>");
                continue;
            }
            "paragraph" => {
                let _ = write!(h, "<p>{}</p>", rich(&d["rich_text"]));
                if !kids.is_empty() {
                    h.push_str("<div style=\"padding-left:1.5em\">");
                    render_blocks(h, kids);
                    h.push_str("</div>");
                }
            }
            "heading_1" | "heading_2" | "heading_3" => {
                let lvl = match ty { "heading_1" => 2, "heading_2" => 3, _ => 4 };
                if d["is_toggleable"] == true {
                    let _ = write!(h, "<details><summary><h{lvl} style=\"display:inline\">{}</h{lvl}></summary><div>", rich(&d["rich_text"]));
                    render_blocks(h, kids);
                    h.push_str("</div></details>");
                } else {
                    let _ = write!(h, "<h{lvl}>{}</h{lvl}>", rich(&d["rich_text"]));
                }
            }
            "to_do" => {
                let done = d["checked"] == true;
                let _ = write!(
                    h,
                    "<div class=\"todo{}\"><input type=checkbox disabled{}><span>{}</span></div>",
                    if done { " done" } else { "" },
                    if done { " checked" } else { "" },
                    rich(&d["rich_text"])
                );
                if !kids.is_empty() {
                    h.push_str("<div style=\"padding-left:1.5em\">");
                    render_blocks(h, kids);
                    h.push_str("</div>");
                }
            }
            "toggle" => {
                let _ = write!(h, "<details><summary>{}</summary><div>", rich(&d["rich_text"]));
                render_blocks(h, kids);
                h.push_str("</div></details>");
            }
            "quote" => {
                let _ = write!(h, "<blockquote>{}", rich(&d["rich_text"]));
                render_blocks(h, kids);
                h.push_str("</blockquote>");
            }
            "callout" => {
                let icon = d["icon"]["emoji"].as_str().unwrap_or("💡");
                let _ = write!(h, "<div class=callout><span>{}</span><div>{}", esc(icon), rich(&d["rich_text"]));
                render_blocks(h, kids);
                h.push_str("</div></div>");
            }
            "code" => {
                let _ = write!(h, "<pre><code>{}</code></pre>", esc(&plain(&d["rich_text"])));
            }
            "divider" => h.push_str("<hr>"),
            "image" => {
                let _ = write!(h, "<p><img src=\"{}\" alt=\"\" loading=lazy></p>", esc(file_url(d)));
            }
            "bookmark" | "embed" | "link_preview" | "video" | "file" | "pdf" => {
                let url = d["url"].as_str().unwrap_or_else(|| file_url(d));
                let _ = write!(h, "<a class=bm href=\"{0}\" target=_blank rel=noopener>{0}</a>", esc(url));
            }
            "child_page" | "child_database" => {
                let _ = write!(h, "<p><span class=rel>{DOC_ICON}{}</span></p>", esc(d["title"].as_str().unwrap_or("Untitled")));
            }
            "table" => {
                h.push_str("<table>");
                for row in kids {
                    h.push_str("<tr>");
                    for cell in row["table_row"]["cells"].as_array().into_iter().flatten() {
                        let _ = write!(h, "<td>{}</td>", rich(cell));
                    }
                    h.push_str("</tr>");
                }
                h.push_str("</table>");
            }
            "column_list" | "column" | "synced_block" => render_blocks(h, kids),
            _ => {
                if let Some(rt) = d.get("rich_text") {
                    let _ = write!(h, "<p>{}</p>", rich(rt));
                }
            }
        }
        i += 1;
    }
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
