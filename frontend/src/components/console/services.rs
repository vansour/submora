use std::collections::HashSet;

use submora_core::is_valid_source_url;
use submora_shared::{
    auth::{CurrentUserResponse, LoginRequest, UpdateAccountRequest},
    users::{
        CreateUserRequest, LinksPayload, UserCacheStatusResponse, UserDiagnosticsResponse,
        UserLinksResponse, UserOrderPayload, UserSummary,
    },
};

use crate::api;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraftLinkStats {
    pub normalized_count: usize,
    pub blank_count: usize,
    pub duplicate_count: usize,
    pub invalid_count: usize,
    pub first_invalid: Option<String>,
    pub normalized_preview: Vec<String>,
}

pub async fn load_current_user() -> Result<Option<CurrentUserResponse>, String> {
    api::get_me().await
}

pub async fn load_users() -> Result<Vec<UserSummary>, String> {
    api::list_users().await
}

pub async fn load_links(username: Option<String>) -> Result<Option<UserLinksResponse>, String> {
    match username {
        Some(username) => api::get_links(&username).await.map(Some),
        None => Ok(None),
    }
}

pub async fn load_diagnostics(
    username: Option<String>,
) -> Result<Option<UserDiagnosticsResponse>, String> {
    match username {
        Some(username) => api::get_diagnostics(&username).await.map(Some),
        None => Ok(None),
    }
}

pub async fn load_cache_status(
    username: Option<String>,
) -> Result<Option<UserCacheStatusResponse>, String> {
    match username {
        Some(username) => api::get_cache_status(&username).await.map(Some),
        None => Ok(None),
    }
}

pub async fn login(username: String, password: String) -> Result<String, String> {
    api::login(&LoginRequest { username, password })
        .await
        .map(|_| "登录成功".to_string())
}

pub async fn logout() -> Result<String, String> {
    api::logout().await.map(|_| "已退出登录".to_string())
}

pub async fn create_user(username: String) -> Result<UserSummary, String> {
    api::create_user(&CreateUserRequest { username }).await
}

pub async fn delete_user(username: String) -> Result<String, String> {
    api::delete_user(&username)
        .await
        .map(|_| "已删除".to_string())
}

pub async fn save_links(username: String, links_text: String) -> Result<UserLinksResponse, String> {
    let payload = LinksPayload {
        links: parse_links(&links_text),
    };
    api::set_links(&username, &payload).await
}

pub async fn update_account(
    current_username: String,
    account_username: String,
    current_password: String,
    new_password: String,
) -> Result<String, String> {
    let new_username = if account_username.trim().is_empty() {
        current_username.clone()
    } else {
        account_username.trim().to_string()
    };
    if new_username == current_username && new_password.is_empty() {
        return Err("请至少修改用户名或填写新密码".to_string());
    }

    api::update_account(&UpdateAccountRequest {
        current_password: Some(current_password),
        new_username,
        new_password,
    })
    .await
    .map(|_| "账户已更新，请重新登录".to_string())
}

pub async fn set_order(order: Vec<String>) -> Result<Vec<String>, String> {
    api::set_order(&UserOrderPayload { order }).await
}

pub async fn refresh_cache(username: String) -> Result<String, String> {
    api::refresh_cache(&username).await.map(|status| {
        if status.state == "empty" {
            format!("{username} 当前没有可缓存的已保存链接")
        } else {
            format!("已刷新 {username} 的缓存")
        }
    })
}

pub async fn clear_cache(username: String) -> Result<String, String> {
    api::clear_cache(&username)
        .await
        .map(|_| format!("已清空 {username} 的缓存"))
}

pub fn parse_links(links_text: &str) -> Vec<String> {
    links_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub fn analyze_links(links_text: &str, preview_limit: usize) -> DraftLinkStats {
    let raw_lines = if links_text.is_empty() {
        Vec::new()
    } else {
        links_text.split('\n').collect::<Vec<_>>()
    };

    let mut seen = HashSet::new();
    let mut blank_count = 0usize;
    let mut duplicate_count = 0usize;
    let mut invalid_count = 0usize;
    let mut first_invalid = None;
    let mut normalized_preview = Vec::new();

    for raw_line in raw_lines.iter() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            blank_count += 1;
            continue;
        }

        if !is_valid_source_url(trimmed) {
            invalid_count += 1;
            if first_invalid.is_none() {
                first_invalid = Some(trimmed.to_string());
            }
            continue;
        }

        if !seen.insert(trimmed.to_string()) {
            duplicate_count += 1;
            continue;
        }

        if normalized_preview.len() < preview_limit {
            normalized_preview.push(trimmed.to_string());
        }
    }

    DraftLinkStats {
        normalized_count: seen.len(),
        blank_count,
        duplicate_count,
        invalid_count,
        first_invalid,
        normalized_preview,
    }
}
