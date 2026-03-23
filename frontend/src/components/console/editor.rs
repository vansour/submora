use dioxus::prelude::*;
use submora_shared::users::{
    LinkDiagnostic, UserCacheStatusResponse, UserDiagnosticsResponse, UserLinksResponse,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use super::{
    actions, services,
    state::{
        FeedbackSignals, LinkDraftState, LoadState, PendingState, RefreshState,
        remember_links_input,
    },
};
use crate::messages::translate_backend_message;

#[component]
pub fn EditorPanel(
    username: String,
    onclose: EventHandler<()>,
    mut editor_username: Signal<Option<String>>,
    mut links_text: Signal<String>,
    drafts: LinkDraftState,
    has_unsaved_changes: bool,
    links_state: LoadState<Option<UserLinksResponse>>,
    diagnostics_state: LoadState<Option<UserDiagnosticsResponse>>,
    cache_state: LoadState<Option<UserCacheStatusResponse>>,
    pending: PendingState,
    feedback: FeedbackSignals,
    refresh: RefreshState,
) -> Element {
    let mut links_error = use_signal(|| None::<String>);
    let mut dragging_index = use_signal(|| None::<usize>);
    let mut drop_target = use_signal(|| None::<usize>);
    let username_for_input = username.clone();
    let username_for_save = username.clone();
    let username_for_copy = username.clone();
    let username_for_delete = username.clone();
    let save_pending = (pending.save_links)();
    let delete_pending = (pending.delete_user)();
    let refresh_cache_pending = (pending.refresh_cache)();
    let clear_cache_pending = (pending.clear_cache)();
    let editor_busy = save_pending || delete_pending;
    let cache_busy = refresh_cache_pending || clear_cache_pending;
    let current_links_text = links_text();
    let draft_stats = services::analyze_links(&current_links_text, 6);
    let rows = link_rows_from_text(&current_links_text);
    let saved_link_count = match &links_state {
        LoadState::Ready(Some(payload)) => payload.links.len(),
        _ => 0,
    };
    let can_refresh_cache =
        !editor_busy && !cache_busy && !has_unsaved_changes && saved_link_count > 0;
    let can_clear_cache = !editor_busy
        && !cache_busy
        && matches!(
            &cache_state,
            LoadState::Ready(Some(status)) if status.state != "empty"
        );
    let local_format_issue = draft_stats.first_invalid.as_ref().map(|invalid| {
        format!(
            "发现 {} 条格式不正确的链接，保存前需要修正。首条问题：{invalid}",
            draft_stats.invalid_count
        )
    });
    {
        let username = username.clone();
        use_effect(move || {
            let _ = &username;
            links_error.set(None);
            dragging_index.set(None);
            drop_target.set(None);
        });
    }

    rsx! {
        article { id: "editor-panel", class: "panel panel--editor editor-panel",
            div { class: "editor-panel__header",
                h2 { class: "editor-panel__title", "{username}" }
                div { class: "button-row editor-panel__header-actions",
                    if has_unsaved_changes {
                        span { class: "tag tag--accent", "未保存" }
                    }
                    button {
                        class: "button button--ghost button--compact",
                        r#type: "button",
                        disabled: editor_busy,
                        onclick: move |_| {
                            let mut next_rows = link_rows_from_text(&links_text());
                            next_rows.push(String::new());
                            commit_link_rows(
                                &username_for_input,
                                &next_rows,
                                links_text,
                                links_error,
                                drafts,
                            );
                        },
                        "新增"
                    }
                    button {
                        class: "button button--ghost button--compact",
                        r#type: "button",
                        onclick: move |_| onclose.call(()),
                        "收起"
                    }
                }
            }
            div { class: "editor-links",
                div { class: "editor-link-list",
                    for (index, value) in rows.iter().cloned().enumerate() {
                        EditorLinkRow {
                            key: "{index}",
                            username: username.clone(),
                            index,
                            value,
                            rows: rows.clone(),
                            links_text,
                            links_error,
                            drafts,
                            disabled: editor_busy,
                            dragging_index,
                            drop_target,
                        }
                    }
                }
            }
            if let Some(message) = links_error() {
                p { class: "field-error", "{message}" }
            } else if let Some(message) = local_format_issue {
                p { class: "field-error", "{message}" }
            }
            EditorLinksStatus { links_state: links_state.clone() }
            EditorRuntimePanel {
                username: username.clone(),
                has_unsaved_changes,
                saved_link_count,
                can_refresh_cache,
                can_clear_cache,
                refresh_cache_pending,
                clear_cache_pending,
                diagnostics_state,
                cache_state,
                pending,
                feedback,
                refresh,
            }
            div { class: "editor-panel__actions",
                div { class: "editor-action-bar",
                    div { class: "button-row editor-panel__primary-actions",
                        button {
                            class: "button button--primary",
                            onclick: move |_| {
                                actions::save_links(
                                    username_for_save.clone(),
                                    links_text(),
                                    links_text,
                                    links_error,
                                    drafts,
                                    pending.save_links,
                                    feedback,
                                    refresh,
                                );
                            },
                            disabled: editor_busy || !has_unsaved_changes || draft_stats.invalid_count > 0,
                            aria_busy: if save_pending { "true" } else { "false" },
                            "保存"
                        }
                        button {
                            class: "button button--ghost",
                            disabled: editor_busy,
                            onclick: move |_| {
                                feedback.clear();
                                let username = username_for_copy.clone();
                                spawn(async move {
                                    match copy_public_route(&username).await {
                                        Ok(_) => feedback.set_status("已复制公共入口链接"),
                                        Err(error) => feedback.set_error(error),
                                    }
                                });
                            },
                            "复制"
                        }
                        button {
                            class: "button button--danger",
                            disabled: editor_busy || cache_busy,
                            aria_busy: if delete_pending { "true" } else { "false" },
                            onclick: move |_| {
                                if confirm_delete_user(&username_for_delete) {
                                    actions::delete_user_and_leave(
                                        username_for_delete.clone(),
                                        editor_username,
                                        links_text,
                                        drafts,
                                        pending.delete_user,
                                        feedback,
                                        refresh,
                                        move || editor_username.set(None),
                                    );
                                }
                            },
                            "删除"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn EditorLinksStatus(links_state: LoadState<Option<UserLinksResponse>>) -> Element {
    match links_state {
        LoadState::Loading => rsx! {
            p { class: "field-hint", "正在加载已保存链接…" }
        },
        LoadState::Error(message) => rsx! {
            p { class: "field-error", "已保存链接加载失败：{message}" }
        },
        _ => rsx! {},
    }
}

#[component]
fn EditorRuntimePanel(
    username: String,
    has_unsaved_changes: bool,
    saved_link_count: usize,
    can_refresh_cache: bool,
    can_clear_cache: bool,
    refresh_cache_pending: bool,
    clear_cache_pending: bool,
    diagnostics_state: LoadState<Option<UserDiagnosticsResponse>>,
    cache_state: LoadState<Option<UserCacheStatusResponse>>,
    pending: PendingState,
    feedback: FeedbackSignals,
    refresh: RefreshState,
) -> Element {
    let username_for_refresh = username.clone();
    let username_for_clear = username.clone();

    let helper_text = if has_unsaved_changes {
        "运行状态基于最近一次已保存的链接配置。若要让缓存和抓取诊断反映当前编辑内容，请先保存。"
    } else if saved_link_count == 0 {
        "当前没有已保存链接，缓存刷新不会产生内容。"
    } else {
        "刷新缓存会重新抓取已保存链接，并同步更新 diagnostics。"
    };

    rsx! {
        section { class: "editor-runtime",
            div { class: "editor-runtime__header",
                div {
                    h3 { "运行状态" }
                    p { class: "field-hint", "{helper_text}" }
                }
                div { class: "button-row editor-runtime__actions",
                    button {
                        class: "button button--ghost button--compact",
                        r#type: "button",
                        disabled: !can_refresh_cache,
                        aria_busy: if refresh_cache_pending { "true" } else { "false" },
                        onclick: move |_| {
                            actions::refresh_user_cache(
                                username_for_refresh.clone(),
                                pending.refresh_cache,
                                feedback,
                                refresh,
                            );
                        },
                        if refresh_cache_pending { "刷新中…" } else { "刷新缓存" }
                    }
                    button {
                        class: "button button--ghost button--compact",
                        r#type: "button",
                        disabled: !can_clear_cache,
                        aria_busy: if clear_cache_pending { "true" } else { "false" },
                        onclick: move |_| {
                            actions::clear_user_cache(
                                username_for_clear.clone(),
                                pending.clear_cache,
                                feedback,
                                refresh,
                            );
                        },
                        if clear_cache_pending { "清理中…" } else { "清空缓存" }
                    }
                }
            }
            div { class: "editor-runtime__grid",
                CacheStatusCard { cache_state }
                DiagnosticsCard { diagnostics_state }
            }
        }
    }
}

#[component]
fn CacheStatusCard(cache_state: LoadState<Option<UserCacheStatusResponse>>) -> Element {
    rsx! {
        section { class: "editor-runtime-card",
            div { class: "editor-runtime-card__head",
                h3 { "缓存状态" }
            }
            match cache_state {
                LoadState::Loading => rsx! {
                    p { class: "field-hint", "正在读取缓存状态…" }
                },
                LoadState::Error(message) => rsx! {
                    p { class: "field-error", "缓存状态加载失败：{message}" }
                },
                LoadState::Ready(Some(status)) => rsx! {
                    div { class: "editor-runtime-card__body",
                        div { class: "editor-runtime-card__summary",
                            strong { "{cache_state_label(&status.state)}" }
                            span { class: "tag", "{status.line_count} 行" }
                        }
                        dl { class: "editor-meta-list",
                            MetaItem { label: "状态", value: cache_state_label(&status.state).to_string() }
                            MetaItem { label: "大小", value: format_bytes(status.body_bytes) }
                            MetaItem { label: "生成时间", value: format_optional_timestamp(status.generated_at) }
                            MetaItem { label: "过期时间", value: format_optional_timestamp(status.expires_at) }
                        }
                    }
                },
                LoadState::Ready(None) => rsx! {
                    p { class: "field-hint", "选择订阅组后显示缓存状态。" }
                },
            }
        }
    }
}

#[component]
fn DiagnosticsCard(diagnostics_state: LoadState<Option<UserDiagnosticsResponse>>) -> Element {
    rsx! {
        section { class: "editor-runtime-card",
            div { class: "editor-runtime-card__head",
                h3 { "抓取诊断" }
            }
            match diagnostics_state {
                LoadState::Loading => rsx! {
                    p { class: "field-hint", "正在读取抓取诊断…" }
                },
                LoadState::Error(message) => rsx! {
                    p { class: "field-error", "抓取诊断加载失败：{message}" }
                },
                LoadState::Ready(Some(payload)) if payload.diagnostics.is_empty() => rsx! {
                    p { class: "field-hint", "当前没有已保存链接。" }
                },
                LoadState::Ready(Some(payload)) => rsx! {
                    div { class: "editor-diagnostic-list",
                        for diagnostic in payload.diagnostics.iter().cloned() {
                            DiagnosticItem {
                                key: "{diagnostic.url}",
                                diagnostic,
                            }
                        }
                    }
                },
                LoadState::Ready(None) => rsx! {
                    p { class: "field-hint", "选择订阅组后显示抓取诊断。" }
                },
            }
        }
    }
}

#[component]
fn DiagnosticItem(diagnostic: LinkDiagnostic) -> Element {
    let detail = diagnostic.detail.as_deref().map(translate_backend_message);
    let status_class = diagnostic_status_class(&diagnostic.status);

    rsx! {
        article { class: "editor-diagnostic-item",
            div { class: "editor-diagnostic-item__head",
                strong { class: "editor-diagnostic-item__url", "{diagnostic.url}" }
                span { class: "tag {status_class}", "{diagnostic_status_label(&diagnostic.status)}" }
            }
            if let Some(detail) = detail {
                p { class: "field-hint", "{detail}" }
            }
            dl { class: "editor-meta-list",
                MetaItem {
                    label: "HTTP",
                    value: diagnostic
                        .http_status
                        .map(|status| status.to_string())
                        .unwrap_or_else(|| "未记录".to_string()),
                }
                MetaItem {
                    label: "类型",
                    value: diagnostic.content_type.unwrap_or_else(|| "未记录".to_string()),
                }
                MetaItem {
                    label: "大小",
                    value: diagnostic
                        .body_bytes
                        .map(format_bytes)
                        .unwrap_or_else(|| "未记录".to_string()),
                }
                MetaItem {
                    label: "重定向",
                    value: diagnostic.redirect_count.to_string(),
                }
                MetaItem {
                    label: "内容",
                    value: if diagnostic.is_html {
                        "HTML".to_string()
                    } else {
                        "原始文本".to_string()
                    },
                }
                MetaItem {
                    label: "抓取时间",
                    value: format_optional_timestamp(diagnostic.fetched_at),
                }
            }
        }
    }
}

#[component]
fn MetaItem(label: &'static str, value: String) -> Element {
    rsx! {
        div { class: "editor-meta-list__item",
            dt { "{label}" }
            dd { "{value}" }
        }
    }
}

#[component]
pub fn EditorEmptyState() -> Element {
    rsx! {
        article { class: "panel panel--empty editor-empty-state",
            div { class: "editor-empty-state__copy",
                h2 { "选择订阅组" }
                p { class: "panel-copy", "从左侧菜单打开后即可直接编辑链接。" }
            }
        }
    }
}

#[component]
fn EditorLinkRow(
    username: String,
    index: usize,
    value: String,
    rows: Vec<String>,
    mut links_text: Signal<String>,
    mut links_error: Signal<Option<String>>,
    drafts: LinkDraftState,
    disabled: bool,
    mut dragging_index: Signal<Option<usize>>,
    mut drop_target: Signal<Option<usize>>,
) -> Element {
    let is_dragging = dragging_index() == Some(index);
    let is_drop_target = drop_target() == Some(index);
    let row_class = if is_drop_target {
        "editor-link-row editor-link-row--drop"
    } else if is_dragging {
        "editor-link-row editor-link-row--dragging"
    } else {
        "editor-link-row"
    };
    let sort_class = if disabled {
        "editor-link-row__sort editor-link-row__sort--disabled"
    } else if is_dragging {
        "editor-link-row__sort editor-link-row__sort--dragging"
    } else {
        "editor-link-row__sort"
    };
    let input_class =
        if !value.trim().is_empty() && !submora_core::is_valid_source_url(value.trim()) {
            "editor-link-row__input editor-link-row__input--error"
        } else {
            "editor-link-row__input"
        };
    let rows_for_input = rows.clone();
    let rows_for_delete = rows.clone();
    let rows_for_drop = rows.clone();
    let username_for_input = username.clone();
    let username_for_delete = username.clone();
    let username_for_drop = username.clone();

    rsx! {
        div {
            class: "{row_class}",
            ondragover: move |event| {
                if disabled {
                    return;
                }

                event.prevent_default();
                drop_target.set(Some(index));
            },
            ondragleave: move |_| {
                if drop_target() == Some(index) {
                    drop_target.set(None);
                }
            },
            ondrop: move |event| {
                if disabled {
                    return;
                }

                event.prevent_default();
                if let Some(from) = dragging_index() {
                    let next_rows = move_link_row_to_index(&rows_for_drop, from, index);
                    commit_link_rows(
                        &username_for_drop,
                        &next_rows,
                        links_text,
                        links_error,
                        drafts,
                    );
                }

                dragging_index.set(None);
                drop_target.set(None);
            },
            div {
                class: "{sort_class}",
                draggable: (!disabled).to_string(),
                tabindex: "0",
                role: "button",
                aria_grabbed: if is_dragging { "true" } else { "false" },
                aria_label: "拖拽调整链接顺序",
                ondragstart: move |event| {
                    if disabled {
                        return;
                    }

                    let transfer = event.data_transfer();
                    let _ = transfer.set_data("text/plain", &index.to_string());
                    transfer.set_effect_allowed("move");
                    dragging_index.set(Some(index));
                    drop_target.set(Some(index));
                },
                ondragend: move |_| {
                    dragging_index.set(None);
                    drop_target.set(None);
                },
                span { class: "editor-link-row__sort-icon", "⋮⋮" }
            }
            input {
                class: "{input_class}",
                disabled: disabled,
                value: "{value}",
                placeholder: "https://example.com/feed",
                aria_invalid: if input_class.contains("--error") { "true" } else { "false" },
                oninput: move |event| {
                    let mut next_rows = rows_for_input.clone();
                    next_rows[index] = event.value();
                    commit_link_rows(
                        &username_for_input,
                        &next_rows,
                        links_text,
                        links_error,
                        drafts,
                    );
                }
            }
            button {
                class: "button button--ghost button--compact editor-link-row__remove",
                r#type: "button",
                disabled: disabled,
                onclick: move |_| {
                    let mut next_rows = rows_for_delete.clone();
                    if next_rows.len() == 1 {
                        next_rows[0].clear();
                    } else {
                        next_rows.remove(index);
                    }
                    commit_link_rows(
                        &username_for_delete,
                        &next_rows,
                        links_text,
                        links_error,
                        drafts,
                    );
                    dragging_index.set(None);
                    drop_target.set(None);
                },
                "删除"
            }
        }
    }
}

fn link_rows_from_text(links_text: &str) -> Vec<String> {
    if links_text.is_empty() {
        return vec![String::new()];
    }

    links_text.split('\n').map(ToOwned::to_owned).collect()
}

fn commit_link_rows(
    username: &str,
    rows: &[String],
    mut links_text: Signal<String>,
    mut links_error: Signal<Option<String>>,
    drafts: LinkDraftState,
) {
    let next_text = rows.join("\n");
    links_error.set(None);
    links_text.set(next_text.clone());
    remember_links_input(username, &next_text, drafts);
}

fn move_link_row_to_index(rows: &[String], from: usize, to: usize) -> Vec<String> {
    if from >= rows.len() || to >= rows.len() || from == to {
        return rows.to_vec();
    }

    let mut next_rows = rows.to_vec();
    let moved = next_rows.remove(from);
    next_rows.insert(to.min(next_rows.len()), moved);
    next_rows
}

fn cache_state_label(state: &str) -> &'static str {
    match state {
        "fresh" => "最新",
        "expired" => "已过期",
        "empty" => "空缓存",
        _ => "未知",
    }
}

fn diagnostic_status_label(status: &str) -> &'static str {
    match status {
        "success" => "成功",
        "error" => "失败",
        "blocked" => "已阻断",
        "pending" => "待抓取",
        _ => "未知",
    }
}

fn diagnostic_status_class(status: &str) -> &'static str {
    match status {
        "success" => "tag--success",
        "error" => "tag--danger",
        "blocked" => "tag--danger",
        "pending" => "tag--cool",
        _ => "",
    }
}

fn format_optional_timestamp(timestamp: Option<i64>) -> String {
    timestamp
        .and_then(|value| OffsetDateTime::from_unix_timestamp(value).ok())
        .and_then(|value| value.format(&Rfc3339).ok())
        .unwrap_or_else(|| "未记录".to_string())
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;

    if bytes < 1024 {
        format!("{bytes} B")
    } else if (bytes as f64) < MB {
        format!("{:.1} KB", bytes as f64 / KB)
    } else {
        format!("{:.1} MB", bytes as f64 / MB)
    }
}

#[cfg(target_arch = "wasm32")]
async fn copy_public_route(username: &str) -> Result<String, String> {
    let window = web_sys::window().ok_or_else(|| "浏览器窗口不可用".to_string())?;
    let origin = window
        .location()
        .origin()
        .map_err(|_| "无法读取当前站点地址".to_string())?;
    let url = format!("{origin}/{username}");
    wasm_bindgen_futures::JsFuture::from(window.navigator().clipboard().write_text(&url))
        .await
        .map_err(js_error_message)?;
    Ok(url)
}

#[cfg(not(target_arch = "wasm32"))]
async fn copy_public_route(username: &str) -> Result<String, String> {
    Ok(format!("/{username}"))
}

#[cfg(target_arch = "wasm32")]
fn js_error_message(error: wasm_bindgen::JsValue) -> String {
    error
        .as_string()
        .unwrap_or_else(|| "复制公共入口链接失败".to_string())
}

#[cfg(target_arch = "wasm32")]
fn confirm_delete_user(username: &str) -> bool {
    web_sys::window()
        .and_then(|window| {
            window
                .confirm_with_message(&format!("确认删除订阅组 {username}？"))
                .ok()
        })
        .unwrap_or(false)
}

#[cfg(not(target_arch = "wasm32"))]
fn confirm_delete_user(_username: &str) -> bool {
    true
}
