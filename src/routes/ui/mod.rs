use axum::response::Html;
use axum::routing::get;
use axum::Router;

pub fn router() -> Router {
    Router::new().route("/", get(index))
}

async fn index() -> Html<&'static str> {
    Html(
        r#"<!DOCTYPE html>
<html lang="en" class="dark">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>JobTracker</title>
    <style>
        body { font-family: system-ui; background: #1a1a2e; color: #e0e0e0; margin: 2rem; }
        h1 { color: #6366f1; }
        a { color: #818cf8; }
        .card { background: #16213e; padding: 1.5rem; border-radius: 0.5rem; margin: 1rem 0; }
    </style>
</head>
<body>
    <h1>JobTracker</h1>
    <div class="card">
        <p>API is running. Full web UI coming in Phase 2.</p>
        <p>Try the API: <a href="/api/v1/jobs">/api/v1/jobs</a> (requires Bearer token)</p>
        <p>Health: <a href="/healthz">/healthz</a> | <a href="/readyz">/readyz</a></p>
    </div>
</body>
</html>"#,
    )
}
