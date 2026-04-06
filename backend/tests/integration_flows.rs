use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
};

use axum::{
    Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{
        Method, Request, StatusCode,
        header::{self},
    },
    response::Response,
    routing::get,
};
use serde::{Serialize, de::DeserializeOwned};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use submora::{
    app,
    config::ServerConfig,
    db, security, session,
    shared::{
        api::{ApiErrorBody, ApiMessage},
        auth::{CsrfTokenResponse, LoginRequest},
        users::{CreateUserRequest, LinksPayload},
    },
    state::AppState,
};
use tokio::{net::TcpListener, sync::Semaphore, task::JoinHandle};
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn public_route_fetches_latest_data_on_every_request() {
    let upstream = UpstreamServer::spawn().await;
    upstream.set_feed("https://one.example/feed\n").await;

    let mut overrides = HashMap::new();
    overrides.insert(
        format!("example.test:{}", upstream.addr.port()),
        vec![upstream.addr],
    );

    let mut app = TestApp::new(TestOptions {
        fetch_host_overrides: overrides,
        ..Default::default()
    })
    .await;

    app.login_ok("admin", "admin").await;
    app.create_user_ok("alpha").await;

    let upstream_url = format!("http://example.test:{}/feed", upstream.addr.port());
    app.set_links_ok("alpha", vec![upstream_url]).await;

    let response = app.send(Method::GET, "/alpha", None, false).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        header_value(&response, header::CACHE_CONTROL.as_str()),
        Some("no-store, no-cache, must-revalidate"),
    );
    assert!(header_value(&response, "x-substore-cache").is_none());
    let body = read_text(response).await;
    assert_eq!(body, "https://one.example/feed");

    upstream.set_feed("https://two.example/feed\n").await;

    let response = app.send(Method::GET, "/alpha", None, false).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = read_text(response).await;
    assert_eq!(body, "https://two.example/feed");
}

#[tokio::test]
async fn public_route_merges_multiple_links_in_saved_order() {
    let upstream = UpstreamServer::spawn().await;
    let mut overrides = HashMap::new();
    overrides.insert(
        format!("example.test:{}", upstream.addr.port()),
        vec![upstream.addr],
    );

    let mut app = TestApp::new(TestOptions {
        fetch_host_overrides: overrides,
        ..Default::default()
    })
    .await;

    app.login_ok("admin", "admin").await;
    app.create_user_ok("ordered").await;
    app.set_links_ok(
        "ordered",
        vec![
            format!("http://example.test:{}/feed", upstream.addr.port()),
            format!("http://example.test:{}/feed-two", upstream.addr.port()),
        ],
    )
    .await;

    let response = app.send(Method::GET, "/ordered", None, false).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = read_text(response).await;
    assert_eq!(body, "https://one.example/feed\n\nhttps://two.example/feed");
}

#[tokio::test]
async fn oversized_upstream_response_is_ignored() {
    let upstream = UpstreamServer::spawn().await;
    let mut overrides = HashMap::new();
    overrides.insert(
        format!("example.test:{}", upstream.addr.port()),
        vec![upstream.addr],
    );

    let mut app = TestApp::new(TestOptions {
        fetch_host_overrides: overrides,
        ..Default::default()
    })
    .await;

    app.login_ok("admin", "admin").await;
    app.create_user_ok("oversized").await;
    app.set_links_ok(
        "oversized",
        vec![format!(
            "http://example.test:{}/oversized",
            upstream.addr.port()
        )],
    )
    .await;

    let response = app.send(Method::GET, "/oversized", None, false).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(read_text(response).await.is_empty());
}

#[tokio::test]
async fn login_rate_limit_returns_retry_after_header() {
    let mut app = TestApp::new(TestOptions {
        login_max_attempts: 2,
        login_lockout_secs: 60,
        ..Default::default()
    })
    .await;

    app.login_expect_status("admin", "wrong-password", StatusCode::UNAUTHORIZED)
        .await;
    app.login_expect_status("admin", "wrong-password", StatusCode::UNAUTHORIZED)
        .await;

    let response = app
        .login_expect_status("admin", "wrong-password", StatusCode::TOO_MANY_REQUESTS)
        .await;
    assert!(header_value(&response, header::RETRY_AFTER.as_str()).is_some());

    let error: ApiErrorBody = read_json(response).await;
    assert!(error.message.contains("too many login attempts"));
}

#[tokio::test]
async fn public_rate_limit_returns_retry_after_header() {
    let mut app = TestApp::new(TestOptions {
        public_max_requests: 1,
        ..Default::default()
    })
    .await;

    app.login_ok("admin", "admin").await;
    app.create_user_ok("limited").await;

    let response = app.send(Method::GET, "/limited", None, false).await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = app.send(Method::GET, "/limited", None, false).await;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(header_value(&response, header::RETRY_AFTER.as_str()).is_some());

    let error: ApiErrorBody = read_json(response).await;
    assert!(error.message.contains("too many public requests"));
}

#[tokio::test]
async fn csrf_token_rotates_on_login_and_stale_tokens_are_rejected() {
    let mut app = TestApp::new(TestOptions::default()).await;

    let anonymous_token = app.fetch_csrf().await;
    let anonymous_cookie = app.session_cookie.clone();
    app.login_ok("admin", "admin").await;
    assert_ne!(anonymous_cookie, app.session_cookie);

    let response = app
        .send_json_with_csrf_token(
            Method::POST,
            "/api/users",
            &CreateUserRequest {
                username: "stale-token".to_string(),
            },
            &anonymous_token,
        )
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let authenticated_token = app.fetch_csrf().await;
    assert_ne!(anonymous_token, authenticated_token);
    app.create_user_ok("fresh-token").await;

    let response = app.send(Method::POST, "/api/auth/logout", None, true).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(app.get_me_status().await, StatusCode::UNAUTHORIZED);

    let response = app
        .send_json_with_csrf_token(
            Method::POST,
            "/api/auth/login",
            &LoginRequest {
                username: "admin".to_string(),
                password: "admin".to_string(),
            },
            &authenticated_token,
        )
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let next_token = app.fetch_csrf().await;
    assert_ne!(authenticated_token, next_token);
    app.login_ok("admin", "admin").await;
}

#[tokio::test]
async fn redirect_to_blocked_target_is_ignored() {
    let upstream = UpstreamServer::spawn().await;
    let mut overrides = HashMap::new();
    overrides.insert(
        format!("example.test:{}", upstream.addr.port()),
        vec![upstream.addr],
    );

    let mut app = TestApp::new(TestOptions {
        fetch_host_overrides: overrides,
        ..Default::default()
    })
    .await;

    app.login_ok("admin", "admin").await;
    app.create_user_ok("blocked").await;
    app.set_links_ok(
        "blocked",
        vec![format!(
            "http://example.test:{}/redirect-local",
            upstream.addr.port()
        )],
    )
    .await;

    let response = app.send(Method::GET, "/blocked", None, false).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(read_text(response).await.is_empty());
}

#[tokio::test]
async fn deleting_user_removes_public_access() {
    let upstream = UpstreamServer::spawn().await;
    let mut overrides = HashMap::new();
    overrides.insert(
        format!("example.test:{}", upstream.addr.port()),
        vec![upstream.addr],
    );

    let mut app = TestApp::new(TestOptions {
        fetch_host_overrides: overrides,
        ..Default::default()
    })
    .await;

    app.login_ok("admin", "admin").await;
    app.create_user_ok("cleanup").await;
    app.set_links_ok(
        "cleanup",
        vec![format!("http://example.test:{}/feed", upstream.addr.port())],
    )
    .await;

    let response = app.send(Method::GET, "/cleanup", None, false).await;
    assert_eq!(response.status(), StatusCode::OK);

    app.delete_user_ok("cleanup").await;

    let response = app.send(Method::GET, "/cleanup", None, false).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

struct TestOptions {
    fetch_host_overrides: HashMap<String, Vec<SocketAddr>>,
    login_max_attempts: usize,
    login_window_secs: u64,
    login_lockout_secs: u64,
    public_max_requests: usize,
    public_window_secs: u64,
}

impl Default for TestOptions {
    fn default() -> Self {
        Self {
            fetch_host_overrides: HashMap::new(),
            login_max_attempts: 5,
            login_window_secs: 300,
            login_lockout_secs: 300,
            public_max_requests: 60,
            public_window_secs: 60,
        }
    }
}

struct TestApp {
    app: Router,
    session_cookie: Option<String>,
    csrf_token: Option<String>,
}

impl TestApp {
    async fn new(options: TestOptions) -> Self {
        let database_path =
            std::env::temp_dir().join(format!("submora-test-{}.db", Uuid::new_v4()));
        let database_url = format!("sqlite://{}?mode=rwc", database_path.display());

        db::prepare_database_dir(&database_url).unwrap();
        let connect_options = SqliteConnectOptions::from_str(&database_url)
            .unwrap()
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(connect_options)
            .await
            .unwrap();

        db::run_migrations(&pool).await.unwrap();
        db::ensure_admin(&pool, "admin", "admin").await.unwrap();

        let config = test_config(database_url, options);
        let session_store = session::build_session_store(pool.clone()).await.unwrap();
        let state = Arc::new(AppState {
            db: pool,
            fetch_semaphore: Arc::new(Semaphore::new(config.concurrent_limit)),
            login_rate_limiter: security::LoginRateLimiter::new(
                config.login_max_attempts,
                config.login_window_secs,
                config.login_lockout_secs,
            ),
            public_rate_limiter: security::PublicRateLimiter::new(
                config.public_max_requests,
                config.public_window_secs,
            ),
            config: config.clone(),
        });

        let app = app::build_router(state.clone())
            .layer(session::build_session_layer(session_store, &config));

        Self {
            app,
            session_cookie: None,
            csrf_token: None,
        }
    }

    async fn send(
        &mut self,
        method: Method,
        path: &str,
        body: Option<String>,
        with_csrf: bool,
    ) -> Response {
        if with_csrf && self.csrf_token.is_none() {
            self.fetch_csrf().await;
        }

        let csrf_token = if with_csrf {
            self.csrf_token.clone()
        } else {
            None
        };

        self.send_raw(method, path, body, csrf_token.as_deref())
            .await
    }

    async fn send_raw(
        &mut self,
        method: Method,
        path: &str,
        body: Option<String>,
        csrf_token: Option<&str>,
    ) -> Response {
        let mut builder = Request::builder().method(method).uri(path);
        if let Some(cookie) = &self.session_cookie {
            builder = builder.header(header::COOKIE, cookie);
        }
        if let Some(csrf_token) = csrf_token {
            builder = builder.header("x-csrf-token", csrf_token);
        }
        if body.is_some() {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
        }

        let mut request = builder.body(Body::from(body.unwrap_or_default())).unwrap();
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 32001))));

        let response = self.app.clone().oneshot(request).await.unwrap();
        self.capture_session_cookie(&response);
        response
    }

    async fn fetch_csrf(&mut self) -> String {
        let response = self
            .send_raw(Method::GET, "/api/auth/csrf", None, None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        let payload: CsrfTokenResponse = read_json(response).await;
        self.csrf_token = Some(payload.token.clone());
        payload.token
    }

    async fn login_expect_status(
        &mut self,
        username: &str,
        password: &str,
        expected_status: StatusCode,
    ) -> Response {
        let response = self
            .send_json(
                Method::POST,
                "/api/auth/login",
                &LoginRequest {
                    username: username.to_string(),
                    password: password.to_string(),
                },
                true,
            )
            .await;
        assert_eq!(response.status(), expected_status);
        self.csrf_token = None;
        response
    }

    async fn login_ok(&mut self, username: &str, password: &str) -> ApiMessage {
        let response = self
            .login_expect_status(username, password, StatusCode::OK)
            .await;
        read_json(response).await
    }

    async fn create_user_ok(&mut self, username: &str) {
        self.create_user_expect_status(username, StatusCode::OK)
            .await;
    }

    async fn create_user_expect_status(&mut self, username: &str, expected_status: StatusCode) {
        let response = self
            .send_json(
                Method::POST,
                "/api/users",
                &CreateUserRequest {
                    username: username.to_string(),
                },
                true,
            )
            .await;
        assert_eq!(response.status(), expected_status);
    }

    async fn set_links_ok(&mut self, username: &str, links: Vec<String>) {
        let response = self
            .send_json(
                Method::PUT,
                &format!("/api/users/{username}/links"),
                &LinksPayload { links },
                true,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    async fn get_me_status(&mut self) -> StatusCode {
        self.send(Method::GET, "/api/auth/me", None, false)
            .await
            .status()
    }

    async fn delete_user_ok(&mut self, username: &str) {
        let response = self
            .send(
                Method::DELETE,
                &format!("/api/users/{username}"),
                None,
                true,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    async fn send_json<T: Serialize>(
        &mut self,
        method: Method,
        path: &str,
        payload: &T,
        with_csrf: bool,
    ) -> Response {
        self.send(
            method,
            path,
            Some(serde_json::to_string(payload).unwrap()),
            with_csrf,
        )
        .await
    }

    async fn send_json_with_csrf_token<T: Serialize>(
        &mut self,
        method: Method,
        path: &str,
        payload: &T,
        csrf_token: &str,
    ) -> Response {
        self.send_raw(
            method,
            path,
            Some(serde_json::to_string(payload).unwrap()),
            Some(csrf_token),
        )
        .await
    }

    fn capture_session_cookie(&mut self, response: &Response) {
        let Some(raw_cookie) = response
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .find_map(|value| value.to_str().ok())
        else {
            return;
        };

        if let Some(cookie) = raw_cookie.split(';').next() {
            self.session_cookie = Some(cookie.to_string());
        }
    }
}

struct UpstreamServer {
    addr: SocketAddr,
    feed: Arc<Mutex<String>>,
    task: JoinHandle<()>,
}

impl UpstreamServer {
    async fn spawn() -> Self {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();
        let redirect_port = addr.port();
        let feed = Arc::new(Mutex::new("https://one.example/feed\n".to_string()));
        let feed_for_route = feed.clone();

        let app = Router::new()
            .route(
                "/feed",
                get(move || {
                    let feed = feed_for_route.clone();
                    async move { feed.lock().unwrap().clone() }
                }),
            )
            .route("/feed-two", get(|| async { "https://two.example/feed\n" }))
            .route("/oversized", get(|| async { "x".repeat(11 * 1024 * 1024) }))
            .route(
                "/redirect-local",
                get(move || async move {
                    (
                        StatusCode::FOUND,
                        [(
                            header::LOCATION,
                            format!("http://127.0.0.1:{redirect_port}/feed"),
                        )],
                    )
                }),
            );
        let task = tokio::spawn(async move {
            axum::serve(listener, app.into_make_service())
                .await
                .unwrap();
        });

        Self { addr, feed, task }
    }

    async fn set_feed(&self, next_body: &str) {
        *self.feed.lock().unwrap() = next_body.to_string();
    }
}

impl Drop for UpstreamServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn test_config(database_url: String, options: TestOptions) -> ServerConfig {
    ServerConfig {
        host: IpAddr::V4(Ipv4Addr::LOCALHOST),
        port: 0,
        web_dist_dir: PathBuf::from("dist"),
        database_url,
        cookie_secure: false,
        session_ttl_minutes: 60,
        session_cleanup_interval_secs: 60,
        trust_proxy_headers: false,
        login_max_attempts: options.login_max_attempts,
        login_window_secs: options.login_window_secs,
        login_lockout_secs: options.login_lockout_secs,
        public_max_requests: options.public_max_requests,
        public_window_secs: options.public_window_secs,
        db_max_connections: 5,
        fetch_timeout_secs: 10,
        fetch_host_overrides: options.fetch_host_overrides,
        concurrent_limit: 4,
        max_links_per_user: 20,
        max_users: 20,
        admin_user: "admin".to_string(),
        admin_password: "admin".to_string(),
        cors_allow_origin: vec!["http://127.0.0.1:8081".to_string()],
    }
}

fn header_value<'a>(response: &'a Response, name: &'static str) -> Option<&'a str> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
}

async fn read_json<T: DeserializeOwned>(response: Response) -> T {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn read_text(response: Response) -> String {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}
