use futures::stream::StreamExt;
use reqwest::{Url, header, redirect::Policy};
use scraper::{ElementRef, Html, Node, Selector};
use sqlx::SqlitePool;
use std::{
    collections::BTreeMap,
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use tracing::warn;

use crate::{
    diagnostics::{self, DiagnosticUpsert},
    error::{ApiError, ApiResult},
    metrics,
};

const MAX_FETCH_BYTES: usize = 10 * 1024 * 1024;
const MAX_BUFFER: usize = 1024 * 1024;
const MAX_REDIRECTS: usize = 5;

#[derive(Clone, Debug)]
pub struct DnsResolver {
    ttl: Duration,
    max_entries: usize,
    cache: Arc<RwLock<HashMap<String, CachedResolution>>>,
    overrides: HashMap<String, Vec<SocketAddr>>,
}

#[derive(Clone, Debug)]
struct CachedResolution {
    addrs: Vec<SocketAddr>,
    expires_at: Instant,
    last_accessed: Instant,
}

#[derive(Clone, Debug)]
struct ResolvedAddrs {
    addrs: Vec<SocketAddr>,
    from_override: bool,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ClientCacheKey {
    host: String,
    resolved_addrs: Vec<SocketAddr>,
}

#[derive(Clone, Debug)]
pub struct PinnedClientPool {
    timeout_secs: u64,
    max_entries: usize,
    clients: Arc<Mutex<HashMap<ClientCacheKey, CachedClient>>>,
}

#[derive(Clone, Debug)]
struct CachedClient {
    client: reqwest::Client,
    last_used: Instant,
}

impl PinnedClientPool {
    pub fn new(timeout_secs: u64, max_entries: usize) -> Self {
        Self {
            timeout_secs,
            max_entries: max_entries.max(1),
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn client_for_target(
        &self,
        target: &ValidatedFetchTarget,
    ) -> Result<reqwest::Client, reqwest::Error> {
        let key = ClientCacheKey {
            host: target.host.clone(),
            resolved_addrs: target.resolved_addrs.clone(),
        };

        let mut clients = self
            .clients
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let now = Instant::now();

        if let Some(entry) = clients.get_mut(&key) {
            entry.last_used = now;
            return Ok(entry.client.clone());
        }

        let client = fetch_client_builder(self.timeout_secs)
            .resolve_to_addrs(&target.host, &target.resolved_addrs)
            .build()?;
        clients.insert(
            key,
            CachedClient {
                client: client.clone(),
                last_used: now,
            },
        );
        evict_oldest_client_if_needed(&mut clients, self.max_entries);
        Ok(client)
    }
}

impl DnsResolver {
    pub fn new(ttl_secs: u64) -> Self {
        Self::with_overrides(ttl_secs, 512, HashMap::new())
    }

    pub fn with_overrides(
        ttl_secs: u64,
        max_entries: usize,
        overrides: HashMap<String, Vec<SocketAddr>>,
    ) -> Self {
        Self {
            ttl: Duration::from_secs(ttl_secs.max(1)),
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_entries: max_entries.max(1),
            overrides,
        }
    }

    async fn resolve_host(
        &self,
        host: &str,
        port: u16,
        url_str: &str,
    ) -> Result<ResolvedAddrs, String> {
        if let Ok(ip) = host.parse::<IpAddr>() {
            return Ok(ResolvedAddrs {
                addrs: vec![SocketAddr::new(ip, port)],
                from_override: false,
            });
        }

        let cache_key = format!("{host}:{port}");
        if let Some(addrs) = self.overrides.get(&cache_key) {
            return Ok(ResolvedAddrs {
                addrs: addrs.clone(),
                from_override: true,
            });
        }

        if let Some(addrs) = self.cached_addrs(&cache_key).await {
            return Ok(ResolvedAddrs {
                addrs,
                from_override: false,
            });
        }

        let resolved_addrs = tokio::net::lookup_host((host, port))
            .await
            .map_err(|_| format!("failed to resolve host: {url_str}"))?
            .collect::<Vec<_>>();

        if resolved_addrs.is_empty() {
            return Err(format!("failed to resolve host: {url_str}"));
        }
        self.cache_resolved_addrs(cache_key, resolved_addrs.clone())
            .await;

        Ok(ResolvedAddrs {
            addrs: resolved_addrs,
            from_override: false,
        })
    }

    async fn cached_addrs(&self, cache_key: &str) -> Option<Vec<SocketAddr>> {
        let now = Instant::now();
        let mut cache = self.cache.write().await;
        prune_expired_dns_entries(&mut cache, now);

        if let Some(entry) = cache.get_mut(cache_key) {
            entry.last_accessed = now;
            return Some(entry.addrs.clone());
        }

        None
    }

    async fn cache_resolved_addrs(&self, cache_key: String, addrs: Vec<SocketAddr>) {
        let now = Instant::now();
        let mut cache = self.cache.write().await;
        prune_expired_dns_entries(&mut cache, now);
        cache.insert(
            cache_key,
            CachedResolution {
                addrs,
                expires_at: now + self.ttl,
                last_accessed: now,
            },
        );
        evict_oldest_dns_entry_if_needed(&mut cache, self.max_entries);
    }
}

#[derive(Clone, Debug)]
struct ValidatedFetchTarget {
    url: Url,
    host: String,
    resolved_addrs: Vec<SocketAddr>,
    host_is_ip_literal: bool,
    from_override: bool,
}

fn fetch_client_builder(timeout_secs: u64) -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .redirect(Policy::none())
        .pool_max_idle_per_host(5)
}

pub fn build_fetch_client(timeout_secs: u64) -> Result<reqwest::Client, reqwest::Error> {
    fetch_client_builder(timeout_secs).build()
}

pub async fn validate_safe_url(resolver: &DnsResolver, url_str: &str) -> Result<(), String> {
    validate_fetch_url(resolver, url_str).await.map(|_| ())
}

pub async fn ensure_safe_url(resolver: &DnsResolver, url_str: &str) -> ApiResult<()> {
    validate_safe_url(resolver, url_str)
        .await
        .map_err(|message| ApiError::validation("url", message))
}

pub struct FetchRuntime<'a> {
    pub pool: &'a SqlitePool,
    pub client: &'a reqwest::Client,
    pub resolver: Arc<DnsResolver>,
    pub pinned_client_pool: Arc<PinnedClientPool>,
    pub semaphore: Arc<tokio::sync::Semaphore>,
    pub concurrent_limit: usize,
}

pub async fn fetch_and_merge_for_user(
    runtime: FetchRuntime<'_>,
    username: &str,
    links: Vec<String>,
) -> String {
    let link_count = links.len();
    let mut fetches = futures::stream::iter(links.into_iter().enumerate().map(|(idx, link)| {
        let client = runtime.client.clone();
        let resolver = runtime.resolver.clone();
        let pinned_client_pool = runtime.pinned_client_pool.clone();
        let semaphore = runtime.semaphore.clone();
        async move {
            let _permit = semaphore
                .acquire()
                .await
                .expect("semaphore should be available");
            fetch_link(&client, &resolver, &pinned_client_pool, idx, link).await
        }
    }))
    .buffer_unordered(runtime.concurrent_limit);

    let mut diagnostics_to_store = Vec::with_capacity(link_count);
    let mut pending_parts = BTreeMap::new();
    let mut next_part_index = 0usize;
    let mut merged = String::new();

    while let Some((idx, result)) = fetches.next().await {
        diagnostics_to_store.push(result.diagnostic);
        pending_parts.insert(idx, result.content);

        while let Some(content) = pending_parts.remove(&next_part_index) {
            if let Some(content) = content {
                append_merged_content(&mut merged, &content);
            }
            next_part_index += 1;
        }
    }

    if let Err(error) =
        diagnostics::store_user_diagnostics(runtime.pool, username, &diagnostics_to_store).await
    {
        warn!(username, error = %error, "failed to persist fetch diagnostics");
    }

    merged
}

async fn fetch_link(
    client: &reqwest::Client,
    resolver: &DnsResolver,
    pinned_client_pool: &PinnedClientPool,
    idx: usize,
    link: String,
) -> (usize, FetchResult) {
    (
        idx,
        fetch_link_body(client, resolver, pinned_client_pool, &link).await,
    )
}

async fn fetch_link_body(
    client: &reqwest::Client,
    resolver: &DnsResolver,
    pinned_client_pool: &PinnedClientPool,
    link: &str,
) -> FetchResult {
    let _timer = metrics::FetchTimer::new();
    let mut current_target = match validate_fetch_url(resolver, link).await {
        Ok(target) => target,
        Err(error) => {
            metrics::record_fetch_request("blocked");
            return failed_attempt(
                link,
                "blocked",
                error,
                AttemptMetadata {
                    http_status: None,
                    content_type: None,
                    body_bytes: None,
                    redirect_count: 0,
                    is_html: false,
                },
            );
        }
    };

    for redirect_count in 0..=MAX_REDIRECTS {
        let response =
            match send_validated_request(client, pinned_client_pool, &current_target).await {
                Ok(response) => response,
                Err(error) => {
                    metrics::record_fetch_request("error");
                    return failed_attempt(
                        link,
                        "error",
                        error,
                        AttemptMetadata {
                            http_status: None,
                            content_type: None,
                            body_bytes: None,
                            redirect_count: redirect_count as u8,
                            is_html: false,
                        },
                    );
                }
            };

        if response.status().is_redirection() {
            if redirect_count == MAX_REDIRECTS {
                warn!(url = %current_target.url, redirects = redirect_count, "too many redirects");
                return failed_attempt(
                    link,
                    "error",
                    format!("too many redirects while fetching {link}: maximum {MAX_REDIRECTS}"),
                    AttemptMetadata {
                        http_status: None,
                        content_type: None,
                        body_bytes: None,
                        redirect_count: redirect_count as u8,
                        is_html: false,
                    },
                );
            }

            let Some(location) = response.headers().get(header::LOCATION) else {
                return failed_attempt(
                    link,
                    "error",
                    format!("redirect missing location header: {}", current_target.url),
                    AttemptMetadata {
                        http_status: None,
                        content_type: None,
                        body_bytes: None,
                        redirect_count: redirect_count as u8,
                        is_html: false,
                    },
                );
            };
            let location = match location.to_str() {
                Ok(location) => location,
                Err(_) => {
                    return failed_attempt(
                        link,
                        "error",
                        format!(
                            "redirect location is not valid utf-8: {}",
                            current_target.url
                        ),
                        AttemptMetadata {
                            http_status: None,
                            content_type: None,
                            body_bytes: None,
                            redirect_count: redirect_count as u8,
                            is_html: false,
                        },
                    );
                }
            };
            current_target =
                match resolve_redirect_url(resolver, &current_target.url, location).await {
                    Ok(target) => target,
                    Err(error) => {
                        return failed_attempt(
                            link,
                            "blocked",
                            error,
                            AttemptMetadata {
                                http_status: None,
                                content_type: None,
                                body_bytes: None,
                                redirect_count: (redirect_count + 1) as u8,
                                is_html: false,
                            },
                        );
                    }
                };
            continue;
        }

        let status_code = response.status().as_u16();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        let is_html = content_type
            .as_deref()
            .map(|value| value.contains("text/html"))
            .unwrap_or(false);

        if !response.status().is_success() {
            return failed_attempt(
                link,
                "error",
                format!(
                    "unexpected response status {} while fetching {}",
                    response.status(),
                    current_target.url
                ),
                AttemptMetadata {
                    http_status: Some(status_code),
                    content_type,
                    body_bytes: None,
                    redirect_count: redirect_count as u8,
                    is_html,
                },
            );
        }

        if let Some(content_length) = response.content_length()
            && content_length > MAX_FETCH_BYTES as u64
        {
            warn!(url = %current_target.url, size = content_length, "content too large");
            return failed_attempt(
                link,
                "error",
                format!(
                    "content too large while fetching {}: {} bytes exceeds {} bytes limit",
                    current_target.url, content_length, MAX_FETCH_BYTES
                ),
                AttemptMetadata {
                    http_status: Some(status_code),
                    content_type,
                    body_bytes: Some(content_length),
                    redirect_count: redirect_count as u8,
                    is_html,
                },
            );
        }

        match read_response_body_limited(response, &current_target.url).await {
            Ok(body) => {
                let body_bytes = body.len() as u64;
                let body = String::from_utf8_lossy(&body).into_owned();
                let content = normalize_fetch_content(body, is_html).await;
                return FetchResult {
                    content,
                    diagnostic: DiagnosticUpsert {
                        source_url: link.to_string(),
                        status: "success".to_string(),
                        detail: Some("Fetch completed successfully".to_string()),
                        http_status: Some(status_code),
                        content_type,
                        body_bytes: Some(body_bytes),
                        redirect_count: redirect_count as u8,
                        is_html,
                    },
                };
            }
            Err(error) => {
                return failed_attempt(
                    link,
                    "error",
                    error,
                    AttemptMetadata {
                        http_status: Some(status_code),
                        content_type,
                        body_bytes: None,
                        redirect_count: redirect_count as u8,
                        is_html,
                    },
                );
            }
        }
    }

    failed_attempt(
        link,
        "error",
        format!("too many redirects while fetching {link}: maximum {MAX_REDIRECTS}"),
        AttemptMetadata {
            http_status: None,
            content_type: None,
            body_bytes: None,
            redirect_count: MAX_REDIRECTS as u8,
            is_html: false,
        },
    )
}

fn failed_attempt(
    link: &str,
    status: &str,
    detail: String,
    metadata: AttemptMetadata,
) -> FetchResult {
    FetchResult {
        content: None,
        diagnostic: DiagnosticUpsert {
            source_url: link.to_string(),
            status: status.to_string(),
            detail: Some(detail),
            http_status: metadata.http_status,
            content_type: metadata.content_type,
            body_bytes: metadata.body_bytes,
            redirect_count: metadata.redirect_count,
            is_html: metadata.is_html,
        },
    }
}

async fn validate_fetch_url(
    resolver: &DnsResolver,
    url_str: &str,
) -> Result<ValidatedFetchTarget, String> {
    let url = Url::parse(url_str).map_err(|_| format!("invalid url: {url_str}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(format!("unsupported scheme: {url_str}"));
    }

    let host = url
        .host_str()
        .ok_or_else(|| format!("missing host: {url_str}"))?
        .to_string();
    let port = url.port_or_known_default().unwrap_or(80);
    let host_is_ip_literal = host.parse::<IpAddr>().is_ok();
    let resolved = resolver.resolve_host(&host, port, url_str).await?;
    if !resolved.from_override {
        for addr in &resolved.addrs {
            if is_forbidden_ip(addr.ip()) {
                return Err(format!("unsafe target: {url_str}"));
            }
        }
    }

    Ok(ValidatedFetchTarget {
        url,
        host,
        resolved_addrs: resolved.addrs,
        host_is_ip_literal,
        from_override: resolved.from_override,
    })
}

async fn resolve_redirect_url(
    resolver: &DnsResolver,
    current_url: &Url,
    location: &str,
) -> Result<ValidatedFetchTarget, String> {
    let next_url = current_url
        .join(location)
        .map_err(|_| format!("invalid redirect target from {current_url}: {location}"))?;
    validate_fetch_url(resolver, next_url.as_str()).await
}

async fn send_validated_request(
    client: &reqwest::Client,
    pinned_client_pool: &PinnedClientPool,
    target: &ValidatedFetchTarget,
) -> Result<reqwest::Response, String> {
    if target.host_is_ip_literal {
        // IP literal 已在 validate_fetch_url 中检查过，直接发送
        return client
            .get(target.url.clone())
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"));
    }

    // TOCTOU 防护：发送前二次验证 resolved IP 是否安全
    // 防止 DNS rebinding 攻击（验证后、请求前 DNS 结果变化）
    if !target.from_override {
        for addr in &target.resolved_addrs {
            if is_forbidden_ip(addr.ip()) {
                warn!(
                    url = %target.url,
                    ip = %addr.ip(),
                    "SSRF TOCTOU detected: resolved IP became unsafe before request"
                );
                return Err(format!(
                    "unsafe resolved IP detected before request: {}",
                    addr.ip()
                ));
            }
        }
    }

    pinned_client_pool
        .client_for_target(target)
        .map_err(|e| format!("failed to create pinned client: {e}"))?
        .get(target.url.clone())
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))
}

async fn read_response_body_limited(
    response: reqwest::Response,
    url: &Url,
) -> Result<Vec<u8>, String> {
    let mut buffer = Vec::new();
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|error| format!("failed to read response body {url}: {error}"))?;
        append_limited(&mut buffer, chunk.as_ref(), MAX_FETCH_BYTES)?;
    }

    Ok(buffer)
}

fn append_limited(buffer: &mut Vec<u8>, chunk: &[u8], max_bytes: usize) -> Result<(), String> {
    let next_len = buffer.len().saturating_add(chunk.len());
    if next_len > max_bytes {
        return Err(format!(
            "content too large: exceeds {} bytes limit while streaming body",
            max_bytes
        ));
    }

    buffer.extend_from_slice(chunk);
    Ok(())
}

fn prune_expired_dns_entries(cache: &mut HashMap<String, CachedResolution>, now: Instant) {
    cache.retain(|_, entry| entry.expires_at > now);
}

fn evict_oldest_dns_entry_if_needed(
    cache: &mut HashMap<String, CachedResolution>,
    max_entries: usize,
) {
    if cache.len() <= max_entries {
        return;
    }

    // 批量驱逐：一次清理超出部分的 20%，避免频繁驱逐
    let excess = cache.len() - max_entries;
    let to_evict = (excess.max(1) as f64 * 1.2).ceil() as usize;

    let mut entries: Vec<_> = cache
        .iter()
        .map(|(key, entry)| (key.clone(), entry.last_accessed))
        .collect();
    entries.sort_by_key(|(_, last_accessed)| *last_accessed);

    for (key, _) in entries.into_iter().take(to_evict) {
        cache.remove(&key);
    }
}

fn evict_oldest_client_if_needed(
    cache: &mut HashMap<ClientCacheKey, CachedClient>,
    max_entries: usize,
) {
    while cache.len() > max_entries {
        let oldest_key = cache
            .iter()
            .min_by_key(|(_, entry)| entry.last_used)
            .map(|(key, _)| key.clone());
        let Some(oldest_key) = oldest_key else {
            break;
        };
        cache.remove(&oldest_key);
    }
}

fn is_forbidden_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => is_forbidden_ipv4(ipv4),
        IpAddr::V6(ipv6) => is_forbidden_ipv6(ipv6),
    }
}

fn is_forbidden_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();

    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_multicast()
        || ip.octets() == [255, 255, 255, 255]
        || a == 0
        || (a == 100 && (64..=127).contains(&b))
        || (a == 192 && b == 0)
        || (a == 192 && b == 168)
        || (a == 198 && (b == 18 || b == 19))
        || a >= 240
}

fn is_forbidden_ipv6(ip: Ipv6Addr) -> bool {
    let first = ip.segments()[0];

    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || (first & 0xfe00) == 0xfc00
        || (first & 0xffc0) == 0xfe80
}

fn html_to_text(input: &str) -> String {
    if input.len() > MAX_FETCH_BYTES {
        warn!(size = input.len(), "html content too large");
        return format!("<!-- HTML too large: {} bytes, truncated -->", input.len());
    }

    let document = Html::parse_document(input);
    let root_selector = Selector::parse(":root").expect("valid root selector");
    let mut buffer = String::with_capacity(input.len().min(MAX_BUFFER));

    if let Some(root) = document.select(&root_selector).next() {
        walk_element_limited(root, &mut buffer, MAX_BUFFER);
    }

    buffer.trim().to_string()
}

fn walk_element_limited(element: ElementRef, buffer: &mut String, max_len: usize) {
    if buffer.len() >= max_len {
        buffer.push_str("\n<!-- content truncated -->");
        return;
    }

    let name = element.value().name();
    if matches!(name, "script" | "style" | "head" | "noscript") {
        return;
    }

    if is_block_element(name) {
        ensure_newlines(buffer, 2);
    } else if name == "br" {
        buffer.push('\n');
    }

    for child in element.children() {
        if buffer.len() >= max_len {
            buffer.push_str("\n<!-- content truncated -->");
            return;
        }

        match child.value() {
            Node::Text(text) => {
                let text = text.trim();
                if !text.is_empty() {
                    if buffer.ends_with(|c: char| !c.is_whitespace()) {
                        buffer.push(' ');
                    }
                    buffer.push_str(text);
                }
            }
            Node::Element(_) => {
                if let Some(child_element) = ElementRef::wrap(child) {
                    walk_element_limited(child_element, buffer, max_len);
                }
            }
            _ => {}
        }
    }

    if is_block_element(name) {
        ensure_newlines(buffer, 2);
    }
}

fn ensure_newlines(buffer: &mut String, count: usize) {
    if buffer.is_empty() {
        return;
    }

    let existing = buffer.chars().rev().take_while(|ch| *ch == '\n').count();
    for _ in existing..count {
        buffer.push('\n');
    }
}

fn is_block_element(tag: &str) -> bool {
    matches!(
        tag,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "canvas"
            | "dd"
            | "div"
            | "dl"
            | "dt"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hr"
            | "li"
            | "main"
            | "nav"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "table"
            | "tfoot"
            | "ul"
            | "video"
            | "tr"
    )
}

struct FetchResult {
    content: Option<String>,
    diagnostic: DiagnosticUpsert,
}

async fn normalize_fetch_content(body: String, is_html: bool) -> Option<String> {
    let normalized = if is_html {
        // 增加 10 秒超时保护，防止恶意 HTML 导致 CPU 阻塞
        match tokio::time::timeout(
            std::time::Duration::from_secs(10),
            tokio::task::spawn_blocking(move || html_to_text(&body)),
        )
        .await
        {
            Ok(Ok(text)) => text,
            Ok(Err(_)) => {
                warn!("html parsing task panicked");
                String::new()
            }
            Err(_) => {
                warn!("html parsing timeout after 10s");
                String::new()
            }
        }
    } else {
        body
    };

    let normalized = normalized.trim().to_string();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn append_merged_content(merged: &mut String, content: &str) {
    if content.is_empty() {
        return;
    }

    if !merged.is_empty() {
        merged.push_str("\n\n");
    }
    merged.push_str(content);
}

struct AttemptMetadata {
    http_status: Option<u16>,
    content_type: Option<String>,
    body_bytes: Option<u64>,
    redirect_count: u8,
    is_html: bool,
}
