#[cfg(test)]
mod tests {
    use crate::api::Api;
    use crate::server::router;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn get_agent_run_exists() {
        let api = Api::new(|| Ok(std::env::temp_dir().join("t091-none/store.sqlite")), || 0);
        let app = router("tok".into(), 4242, api);
        let request = Request::builder()
            .uri("/api/agent-run")
            .header("host", "127.0.0.1:4242")
            .header("x-dashboard-token", "tok")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), 200, "GET /api/agent-run");
    }
}
