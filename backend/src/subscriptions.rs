use futures::stream::StreamExt;
use reqwest::{Url, header, redirect::Policy};
use scraper::{ElementRef, Html, Node, Selector};
use std::{
    collections::{BTreeMap, HashMap},
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};
use tokio::sync::Semaphore;
use tracing::warn;

use crate::{
    error::{ApiError, ApiResult},
    metrics,
};

const MAX_FETCH_BYTES: usize = 10 * 1024 * 1024;
const MAX_BUFFER: usize = 1024 * 1024;
const MAX_REDIRECTS: usize = 5;

#[derive(Clone, Debug)]
struct ResolvedAddrs {
    addrs: Vec<SocketAddr>,
    from_override: bool,
}

#[derive(Clone, Debug)]
struct ValidatedFetchTarget {
    url: Url,
    host: String,
    resolved_addrs: Vec<SocketAddr>,
    host_is_ip_literal: bool,
    from_override: bool,
}

pub struct FetchRuntime<'a> {
    pub fetch_timeout_secs: u64,
    pub fetch_host_overrides: &'a HashMap<String, Vec<SocketAddr>>,
    pub semaphore: Arc<Semaphore>,
    pub concurrent_limit: usize,
}

fn fetch_client_builder(timeout_secs: u64) -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .redirect(Policy::none())
        .pool_max_idle_per_host(0)
}

pub async fn validate_safe_url(
    fetch_host_overrides: &HashMap<String, Vec<SocketAddr>>,
    url_str: &str,
) -> Result<(), String> {
    validate_fetch_url(fetch_host_overrides, url_str)
        .await
        .map(|_| ())
}

pub async fn ensure_safe_url(
    fetch_host_overrides: &HashMap<String, Vec<SocketAddr>>,
    url_str: &str,
) -> ApiResult<()> {
    validate_safe_url(fetch_host_overrides, url_str)
        .await
        .map_err(|message| ApiError::validation("url", message))
}

pub async fn fetch_and_merge_for_user(runtime: FetchRuntime<'_>, links: Vec<String>) -> String {
    let mut fetches = futures::stream::iter(links.into_iter().enumerate().map(|(idx, link)| {
        let semaphore = runtime.semaphore.clone();
        let fetch_host_overrides = runtime.fetch_host_overrides;
        async move {
            let _permit = semaphore
                .acquire()
                .await
                .expect("semaphore should be available");
            (
                idx,
                fetch_link_body(runtime.fetch_timeout_secs, fetch_host_overrides, &link).await,
            )
        }
    }))
    .buffer_unordered(runtime.concurrent_limit);

    let mut pending_parts = BTreeMap::new();
    let mut next_part_index = 0usize;
    let mut merged = String::new();

    while let Some((idx, content)) = fetches.next().await {
        pending_parts.insert(idx, content);

        while let Some(content) = pending_parts.remove(&next_part_index) {
            if let Some(content) = content {
                append_merged_content(&mut merged, &content);
            }
            next_part_index += 1;
        }
    }

    merged
}

async fn fetch_link_body(
    fetch_timeout_secs: u64,
    fetch_host_overrides: &HashMap<String, Vec<SocketAddr>>,
    link: &str,
) -> Option<String> {
    let _timer = metrics::FetchTimer::new();
    let mut current_target = match validate_fetch_url(fetch_host_overrides, link).await {
        Ok(target) => target,
        Err(error) => {
            metrics::record_fetch_request("blocked");
            warn!(url = link, error, "blocked upstream fetch target");
            return None;
        }
    };

    for redirect_count in 0..=MAX_REDIRECTS {
        let response = match send_validated_request(fetch_timeout_secs, &current_target).await {
            Ok(response) => response,
            Err(error) => {
                metrics::record_fetch_request("error");
                warn!(url = link, error, "upstream request failed");
                return None;
            }
        };

        if response.status().is_redirection() {
            if redirect_count == MAX_REDIRECTS {
                metrics::record_fetch_request("error");
                warn!(url = link, redirects = redirect_count, "too many redirects");
                return None;
            }

            let Some(location) = response.headers().get(header::LOCATION) else {
                metrics::record_fetch_request("error");
                warn!(url = link, "redirect missing location header");
                return None;
            };
            let location = match location.to_str() {
                Ok(location) => location,
                Err(_) => {
                    metrics::record_fetch_request("error");
                    warn!(url = link, "redirect location is not valid utf-8");
                    return None;
                }
            };
            current_target =
                match resolve_redirect_url(fetch_host_overrides, &current_target.url, location)
                    .await
                {
                    Ok(target) => target,
                    Err(error) => {
                        metrics::record_fetch_request("blocked");
                        warn!(url = link, error, "redirect target blocked");
                        return None;
                    }
                };
            continue;
        }

        if !response.status().is_success() {
            metrics::record_fetch_request("error");
            warn!(
                url = %current_target.url,
                status = %response.status(),
                "upstream returned non-success status"
            );
            return None;
        }

        if let Some(content_length) = response.content_length()
            && content_length > MAX_FETCH_BYTES as u64
        {
            metrics::record_fetch_request("error");
            warn!(
                url = %current_target.url,
                size = content_length,
                "upstream content too large"
            );
            return None;
        }

        let is_html = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.contains("text/html"))
            .unwrap_or(false);

        match read_response_body_limited(response, &current_target.url).await {
            Ok(body) => {
                metrics::record_fetch_request("success");
                let body = String::from_utf8_lossy(&body).into_owned();
                return normalize_fetch_content(body, is_html).await;
            }
            Err(error) => {
                metrics::record_fetch_request("error");
                warn!(url = %current_target.url, error, "failed to read upstream body");
                return None;
            }
        }
    }

    metrics::record_fetch_request("error");
    None
}

async fn validate_fetch_url(
    fetch_host_overrides: &HashMap<String, Vec<SocketAddr>>,
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
    let resolved = resolve_host(fetch_host_overrides, &host, port, url_str).await?;

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

async fn resolve_host(
    fetch_host_overrides: &HashMap<String, Vec<SocketAddr>>,
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

    let override_key = format!("{host}:{port}");
    if let Some(addrs) = fetch_host_overrides.get(&override_key) {
        return Ok(ResolvedAddrs {
            addrs: addrs.clone(),
            from_override: true,
        });
    }

    let resolved_addrs = tokio::net::lookup_host((host, port))
        .await
        .map_err(|_| format!("failed to resolve host: {url_str}"))?
        .collect::<Vec<_>>();

    if resolved_addrs.is_empty() {
        return Err(format!("failed to resolve host: {url_str}"));
    }

    Ok(ResolvedAddrs {
        addrs: resolved_addrs,
        from_override: false,
    })
}

async fn resolve_redirect_url(
    fetch_host_overrides: &HashMap<String, Vec<SocketAddr>>,
    current_url: &Url,
    location: &str,
) -> Result<ValidatedFetchTarget, String> {
    let next_url = current_url
        .join(location)
        .map_err(|_| format!("invalid redirect target from {current_url}: {location}"))?;
    validate_fetch_url(fetch_host_overrides, next_url.as_str()).await
}

async fn send_validated_request(
    fetch_timeout_secs: u64,
    target: &ValidatedFetchTarget,
) -> Result<reqwest::Response, String> {
    if !target.from_override {
        for addr in &target.resolved_addrs {
            if is_forbidden_ip(addr.ip()) {
                warn!(
                    url = %target.url,
                    ip = %addr.ip(),
                    "unsafe resolved IP detected before request"
                );
                return Err(format!(
                    "unsafe resolved IP detected before request: {}",
                    addr.ip()
                ));
            }
        }
    }

    let client = if target.host_is_ip_literal {
        fetch_client_builder(fetch_timeout_secs)
            .build()
            .map_err(|error| format!("failed to create request client: {error}"))?
    } else {
        fetch_client_builder(fetch_timeout_secs)
            .resolve_to_addrs(&target.host, &target.resolved_addrs)
            .build()
            .map_err(|error| format!("failed to create resolved request client: {error}"))?
    };

    client
        .get(target.url.clone())
        .send()
        .await
        .map_err(|error| format!("request failed: {error}"))
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
        return String::new();
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

async fn normalize_fetch_content(body: String, is_html: bool) -> Option<String> {
    let normalized = if is_html {
        match tokio::time::timeout(
            Duration::from_secs(10),
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
