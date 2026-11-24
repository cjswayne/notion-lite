use serde_json::{json, Value};

pub const TASKS_DS: &str = "4c1ee288-534e-410f-a095-e01e1a0be674";
pub const ME: &str = "e2490ed9-4a99-4203-b2f3-eca46ed42c7c";

const DATIFY: &str = "59c6169f-08fb-4a17-901e-4c375bf18d7b";
const CLIENT_373: &str = "373bf8c7-4be7-4592-b67a-d0f16aaee7b3";
const CLIENT_175: &str = "175fa1fe-962a-4349-979e-daf298a471c8";
const CLIENT_029: &str = "029dda0a-f909-42ea-a382-62e8082d7c77";

pub const STATUSES: &[&str] = &[
    "Up Next",
    "In Progress - JadePuma",
    "In Progress / To Do - Client",
    "Completed",
    "Deferred",
];

pub struct View {
    pub slug: &'static str,
    pub name: &'static str,
    pub filter: fn() -> Value,
    pub sub_group_client: bool,
}

fn me() -> Value {
    json!({"property": "Agent", "people": {"contains": ME}})
}
fn client_is(id: &str) -> Value {
    json!({"property": "Client", "relation": {"contains": id}})
}
fn client_not(id: &str) -> Value {
    json!({"property": "Client", "relation": {"does_not_contain": id}})
}
fn tag(t: &str) -> Value {
    json!({"property": "Tags", "multi_select": {"contains": t}})
}
fn only_me() -> Value {
    json!({"and": [me()]})
}
fn priority_tag() -> Value {
    json!({"and": [me(), tag("Priority")]})
}

const fn v(slug: &'static str, name: &'static str, filter: fn() -> Value) -> View {
    View { slug, name, filter, sub_group_client: false }
}

/// Same tabs, order and filters as the "View of Tasks" board on the Charles page.
pub const VIEWS: &[View] = &[
    v("priority", "Priority", || json!({"or": [{"and": [me(), tag("Priority")]}, {"and": [me(), tag("Urgent")]}]})),
    v("datify", "Datify", || json!({"and": [me(), client_is(DATIFY)]})),
    v("apps", "Apps", only_me),
    v("apt-1", "APT (1)", only_me),
    View { slug: "bugs", name: "Bugs", filter: only_me, sub_group_client: true },
    v("unassigned", "Unassigned", || json!({"and": [me(), client_not(CLIENT_373), client_not(DATIFY)]})),
    v("clients", "Clients", || json!({"and": [me(), client_not(DATIFY), client_not(CLIENT_373), client_not(CLIENT_175)]})),
    v("client-priority", "Client Priority", priority_tag),
    v("priority-2", "Priority", priority_tag),
    v("charles", "Charles", only_me),
    v("calendar", "Calendar", only_me),
    v("badgezilla", "Badgezilla", only_me),
    v("private-apps", "Private Apps", only_me),
    v("flex-theme-sections", "Flex Theme Sections", only_me),
    v("section-tasks", "Section Tasks", || json!({"and": [me(), {"property": "Name", "title": {"contains": "JPFT"}}]})),
    v("linear-shopping-exps", "Linear Shopping Exps", only_me),
    v("app-admin-interface", "App Admin Interface", only_me),
    v("taxonomy-manager", "Taxonomy Manager", only_me),
    v("all", "All", || {
        json!({"or": [
            {"and": [me(), client_not(CLIENT_029), client_not(DATIFY), client_not(CLIENT_373)]},
            {"and": [me(), client_is(CLIENT_029), tag("Bug")]}
        ]})
    }),
];

pub fn find(slug: &str) -> Option<&'static View> {
    VIEWS.iter().find(|v| v.slug == slug)
}

/// Views with identical filters share one cache entry, so Notion is queried once per distinct filter.
pub fn canonical(slug: &str) -> &'static str {
    let Some(v) = find(slug) else { return "" };
    let f = (v.filter)();
    VIEWS.iter().find(|o| (o.filter)() == f).map(|o| o.slug).unwrap_or(v.slug)
}

/// Pushes leaves into every inner `and` so filters stay within Notion's two-level nesting limit.
pub fn with_leaves(mut f: Value, leaves: &[Value]) -> Value {
    if let Some(arr) = f.get_mut("and").and_then(Value::as_array_mut) {
        arr.extend_from_slice(leaves);
    } else if let Some(arr) = f.get_mut("or").and_then(Value::as_array_mut) {
        for child in arr.iter_mut() {
            if let Some(inner) = child.get_mut("and").and_then(Value::as_array_mut) {
                inner.extend_from_slice(leaves);
            } else {
                let mut v = vec![child.take()];
                v.extend_from_slice(leaves);
                *child = json!({ "and": v });
            }
        }
    }
    f
}

pub fn open_filter(v: &View) -> Value {
    with_leaves(
        (v.filter)(),
        &[
            json!({"property": "Status", "select": {"does_not_equal": "Completed"}}),
            json!({"property": "Status", "select": {"does_not_equal": "Deferred"}}),
        ],
    )
}

pub fn done_filter(v: &View, since: &str) -> Value {
    with_leaves(
        (v.filter)(),
        &[
            json!({"property": "Status", "select": {"equals": "Completed"}}),
            json!({"timestamp": "last_edited_time", "last_edited_time": {"on_or_after": since}}),
        ],
    )
}
