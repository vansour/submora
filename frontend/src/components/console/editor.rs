use dioxus::prelude::*;

use super::{
    actions, services,
    state::{FeedbackSignals, LinkDraftState, PendingState, RefreshState, remember_links_input},
};

#[component]
pub fn EditorPanel(
    username: String,
    onclose: EventHandler<()>,
    mut editor_username: Signal<Option<String>>,
    mut links_text: Signal<String>,
    drafts: LinkDraftState,
    has_unsaved_changes: bool,
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
    let editor_busy = save_pending || delete_pending;
    let current_links_text = links_text();
    let draft_stats = services::analyze_links(&current_links_text, 6);
    let rows = link_rows_from_text(&current_links_text);
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
                            onclick: move |_| match copy_public_route(&username_for_copy) {
                                Ok(_) => feedback.set_status("已复制公共入口链接"),
                                Err(error) => feedback.set_error(error),
                            },
                            "复制"
                        }
                        button {
                            class: "button button--danger",
                            disabled: editor_busy,
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

#[cfg(target_arch = "wasm32")]
fn copy_public_route(username: &str) -> Result<String, String> {
    let window = web_sys::window().ok_or_else(|| "浏览器窗口不可用".to_string())?;
    let origin = window
        .location()
        .origin()
        .map_err(|_| "无法读取当前站点地址".to_string())?;
    let url = format!("{origin}/{username}");
    let _ = window.navigator().clipboard().write_text(&url);
    Ok(url)
}

#[cfg(not(target_arch = "wasm32"))]
fn copy_public_route(username: &str) -> Result<String, String> {
    Ok(format!("/{username}"))
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
