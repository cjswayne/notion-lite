# notion-lite

A tiny, fast, server-rendered view of my Notion task boards. One Rust binary, ~2 KB pages, no JS framework.

## Views
Mine, Priority, Datify, Clients, Unassigned, Bugs, All. Same filters as the "View of Tasks" board views, grouped by Status. Filters live in `src/views.rs`.

## Features
- Every board is cached and refreshed from Notion every 60s in the background, so page loads never wait on Notion.
- Change a task's Status from its card (writes back to Notion).
- `/` to filter cards, "show done" adds tasks completed in the last 14 days.
- Dark mode, mobile layout, gzip/brotli.

## Run
```
NOTION_TOKEN=ntn_xxx DASH_USER=charles DASH_PASSWORD=something cargo run --release
```
Env vars:
- `NOTION_TOKEN` (required): internal integration secret. Share the Tasks and Clients databases with the integration.
- `DASH_USER` / `DASH_PASSWORD`: HTTP basic auth. Without them the site is public, so always set them when deployed.
- `PORT`: default 8080.

## Docker
```
docker build -t notion-lite . && docker run -p 8080:8080 -e NOTION_TOKEN=... -e DASH_USER=... -e DASH_PASSWORD=... notion-lite
```
