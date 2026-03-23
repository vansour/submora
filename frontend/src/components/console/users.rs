use dioxus::prelude::*;
use submora_shared::users::UserSummary;

use super::{
    actions,
    state::{FeedbackSignals, LoadState, PendingState, RefreshState},
};

#[derive(Clone, Debug, PartialEq)]
enum SortDropTarget {
    Row(String),
}

#[component]
pub fn UsersPanel(
    mut create_username: Signal<String>,
    users_state: LoadState<Vec<UserSummary>>,
    selected_username: Option<String>,
    mut editor_username: Signal<Option<String>>,
    pending: PendingState,
    feedback: FeedbackSignals,
    refresh: RefreshState,
) -> Element {
    let dragging_username = use_signal(|| None::<String>);
    let drop_target = use_signal(|| None::<SortDropTarget>);
    let user_list = match &users_state {
        LoadState::Ready(users) => users.clone(),
        _ => Vec::new(),
    };
    let selected = selected_username.clone();
    let create_pending = (pending.create_user)();
    let reorder_pending = (pending.reorder_users)();

    rsx! {
        aside { class: "panel users-panel", aria_label: "订阅组菜单",
            div { class: "section-head users-panel__head",
                div {
                    h2 { "订阅组" }
                }
            }
            div { class: "users-panel__tools",
                form {
                    class: "inline-form users-panel__create",
                    onsubmit: move |event| {
                        event.prevent_default();
                        actions::create_user_and_open(
                            create_username(),
                            create_username,
                            pending.create_user,
                            feedback,
                            refresh,
                            move |username| editor_username.set(Some(username)),
                        );
                    },
                    input {
                        class: "users-panel__input",
                        disabled: create_pending,
                        value: "{create_username()}",
                        oninput: move |event| create_username.set(event.value()),
                        placeholder: "alpha-feed"
                    }
                    button {
                        class: "button button--primary button--compact",
                        r#type: "submit",
                        disabled: create_pending,
                        aria_busy: if create_pending { "true" } else { "false" },
                        if create_pending { "创建中…" } else { "新建" }
                    }
                }
            }
            match users_state.clone() {
                LoadState::Loading => rsx! {
                    p { class: "muted", "正在加载订阅组…" }
                },
                LoadState::Error(message) => rsx! {
                    div { class: "form-stack",
                        p { class: "field-error", "订阅组加载失败：{message}" }
                        button {
                            class: "button button--ghost button--compact",
                            r#type: "button",
                            onclick: move |_| refresh.bump_users(),
                            "重试"
                        }
                    }
                },
                LoadState::Ready(_) if user_list.is_empty() => rsx! {
                    div { class: "empty-state",
                        strong { "还没有订阅组" }
                    }
                },
                LoadState::Ready(_) => rsx! {
                    div { class: "user-list",
                        for user in user_list.iter().cloned() {
                            UserRow {
                                key: "{user.username}",
                                user,
                                users: user_list.clone(),
                                selected: selected.clone(),
                                editor_username,
                                dragging_username,
                                drop_target,
                                pending,
                                feedback,
                                refresh,
                                reorder_pending,
                            }
                        }
                    }
                },
            }
        }
    }
}

#[component]
fn UserRow(
    user: UserSummary,
    users: Vec<UserSummary>,
    selected: Option<String>,
    mut editor_username: Signal<Option<String>>,
    mut dragging_username: Signal<Option<String>>,
    mut drop_target: Signal<Option<SortDropTarget>>,
    pending: PendingState,
    feedback: FeedbackSignals,
    refresh: RefreshState,
    reorder_pending: bool,
) -> Element {
    let is_selected = selected.as_deref() == Some(user.username.as_str());
    let is_dragging = dragging_username().as_deref() == Some(user.username.as_str());
    let is_drop_target = drop_target() == Some(SortDropTarget::Row(user.username.clone()));
    let username = user.username.clone();
    let username_for_drag = username.clone();
    let username_for_drag_target = username.clone();
    let username_for_drop_target = username.clone();
    let username_for_leave = username.clone();
    let username_for_up = username.clone();
    let username_for_down = username.clone();
    let users_for_drop = users.clone();
    let users_for_up = users.clone();
    let users_for_down = users.clone();
    let row_class = if is_drop_target {
        "user-card user-card--drop"
    } else if is_dragging {
        "user-card user-card--dragging"
    } else if is_selected {
        "user-card user-card--selected"
    } else {
        "user-card"
    };
    let edit_button_class = if is_selected {
        "button button--primary button--compact"
    } else {
        "button button--ghost button--compact"
    };
    let sort_class = if reorder_pending {
        "user-card__sort user-card__sort--disabled"
    } else if is_dragging {
        "user-card__sort user-card__sort--dragging"
    } else {
        "user-card__sort"
    };

    rsx! {
        article {
            class: "{row_class}",
            ondragover: move |event| {
                if reorder_pending {
                    return;
                }

                event.prevent_default();
                drop_target.set(Some(SortDropTarget::Row(
                    username_for_drag_target.clone(),
                )));
            },
            ondragleave: move |_| {
                if drop_target() == Some(SortDropTarget::Row(username_for_leave.clone())) {
                    drop_target.set(None);
                }
            },
            ondrop: move |event| {
                if reorder_pending {
                    return;
                }

                event.prevent_default();
                if let Some(dragged) = dragging_username()
                    && let Some(order) =
                        actions::move_username_before(
                            &users_for_drop,
                            &dragged,
                            &username_for_drop_target,
                        )
                {
                    actions::submit_user_order(
                        order,
                        pending.reorder_users,
                        feedback,
                        refresh,
                    );
                }

                dragging_username.set(None);
                drop_target.set(None);
            },
            div { class: "user-card__meta",
                strong { "{user.username}" }
            }
            div { class: "user-card__buttons",
                button {
                    class: "{edit_button_class}",
                    r#type: "button",
                    onclick: move |_| editor_username.set(Some(username.clone())),
                    "打开"
                }
            }
            div {
                class: "{sort_class}",
                draggable: (!reorder_pending).to_string(),
                tabindex: "0",
                role: "button",
                aria_grabbed: if is_dragging { "true" } else { "false" },
                aria_label: "拖拽调整 {user.username} 排序",
                ondragstart: move |event| {
                    if reorder_pending {
                        return;
                    }

                    let transfer = event.data_transfer();
                    let _ = transfer.set_data("text/plain", &username_for_drag);
                    transfer.set_effect_allowed("move");
                    dragging_username.set(Some(username_for_drag.clone()));
                    drop_target.set(Some(SortDropTarget::Row(username_for_drag.clone())));
                },
                ondragend: move |_| {
                    dragging_username.set(None);
                    drop_target.set(None);
                },
                onkeydown: move |event| {
                    if reorder_pending {
                        return;
                    }

                    let next_order = match event.key() {
                        Key::ArrowUp => {
                            event.prevent_default();
                            actions::reordered_usernames(&users_for_up, &username_for_up, -1)
                        }
                        Key::ArrowDown => {
                            event.prevent_default();
                            actions::reordered_usernames(&users_for_down, &username_for_down, 1)
                        }
                        _ => None,
                    };

                    if let Some(order) = next_order {
                        actions::submit_user_order(
                            order,
                            pending.reorder_users,
                            feedback,
                            refresh,
                        );
                    }
                },
                span { class: "user-card__sort-icon", "⋮⋮" }
            }
        }
    }
}
