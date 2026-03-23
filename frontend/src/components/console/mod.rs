mod actions;
mod auth;
mod editor;
mod services;
mod state;
mod users;

use dioxus::prelude::*;

use crate::components::shell::AppShell;
use auth::{AccountPanel, ControlPlanePanel, LoginPanel};
use editor::{EditorEmptyState, EditorPanel};
use state::{
    FeedbackSignals, LoadState, has_unsaved_links, optional_resource_state, resource_state,
    sync_links_text, use_console_resources, use_feedback_signals, use_link_draft_state,
    use_pending_state, use_refresh_state,
};
use users::UsersPanel;

#[component]
pub fn AdminConsole() -> Element {
    let login_username = use_signal(String::new);
    let login_password = use_signal(String::new);
    let create_username = use_signal(String::new);
    let links_text = use_signal(String::new);
    let mut account_username = use_signal(String::new);
    let mut account_current_password = use_signal(String::new);
    let mut account_new_password = use_signal(String::new);
    let mut selected_username = use_signal(|| None::<String>);
    let mut account_modal_open = use_signal(|| false);

    let feedback = use_feedback_signals();
    let link_drafts = use_link_draft_state();
    let pending = use_pending_state();
    let refresh = use_refresh_state();
    let resources = use_console_resources(selected_username(), refresh);
    sync_links_text(
        selected_username(),
        links_text,
        resources.links_resource,
        link_drafts,
    );

    let current_user = optional_resource_state(&resources.auth_resource);
    let users_state = resource_state(&resources.users_resource);
    let links_state = optional_resource_state(&resources.links_resource);
    let diagnostics_state = optional_resource_state(&resources.diagnostics_resource);
    let cache_state = optional_resource_state(&resources.cache_resource);
    let current_links_text = links_text();
    let has_unsaved_changes = has_unsaved_links(
        selected_username().as_deref(),
        &current_links_text,
        link_drafts,
    );
    let authenticated_username = match &current_user {
        LoadState::Ready(Some(user)) => Some(user.username.clone()),
        _ => None,
    };
    let active_username = selected_username();
    let is_authenticated = matches!(&current_user, LoadState::Ready(Some(_)));

    {
        let authenticated_username = authenticated_username.clone();
        use_effect(use_reactive!(|(authenticated_username,)| {
            let _ = &authenticated_username;
            account_modal_open.set(false);
            account_username.set(String::new());
            account_current_password.set(String::new());
            account_new_password.set(String::new());
        }));
    }

    let console_view = match current_user.clone() {
        LoadState::Ready(Some(user)) => {
            let current_username = user.username.clone();
            rsx! {
                div { class: "console-frame",
                    ControlPlanePanel {
                        username: user.username,
                        selected_username: active_username.clone(),
                        account_modal_open,
                        links_text,
                        pending,
                        feedback,
                        refresh,
                    }
                    section { class: "console-workbench",
                        UsersPanel {
                            create_username,
                            users_state: users_state.clone(),
                            selected_username: selected_username(),
                            editor_username: selected_username,
                            pending,
                            feedback,
                            refresh,
                        }
                        if let Some(username) = active_username.clone() {
                            EditorPanel {
                                username,
                                onclose: move |_| selected_username.set(None),
                                editor_username: selected_username,
                                links_text,
                                drafts: link_drafts,
                                has_unsaved_changes,
                                links_state: links_state.clone(),
                                diagnostics_state: diagnostics_state.clone(),
                                cache_state: cache_state.clone(),
                                pending,
                                feedback,
                                refresh,
                            }
                        } else {
                            EditorEmptyState {}
                        }
                    }
                }
                if account_modal_open() {
                    ConsoleModal {
                        label: "管理员账户",
                        size_class: "console-modal--narrow",
                        onclose: move |_| account_modal_open.set(false),
                        AccountPanel {
                            account_username,
                            account_current_password,
                            account_new_password,
                            current_username,
                            onclose: move |_| account_modal_open.set(false),
                            pending,
                            feedback,
                            refresh,
                        }
                    }
                }
            }
        }
        LoadState::Ready(None) => rsx! {
            LoginPanel {
                login_username,
                login_password,
                pending,
                feedback,
                refresh,
            }
        },
        LoadState::Loading => rsx! {
            ConsoleStatePanel {
                title: "恢复会话中",
                message: "正在读取当前登录状态。".to_string(),
            }
        },
        LoadState::Error(message) => rsx! {
            ConsoleStatePanel {
                title: "无法加载会话",
                message,
                action_label: Some("重试"),
                onaction: move |_| refresh.bump_auth(),
            }
        },
    };

    rsx! {
        AppShell {
            compact: !is_authenticated,
            ToastViewport { feedback }
            {console_view}
        }
    }
}

#[component]
fn ConsoleStatePanel(
    title: &'static str,
    message: String,
    action_label: Option<&'static str>,
    onaction: Option<EventHandler<()>>,
) -> Element {
    rsx! {
        article { class: "panel panel--hero auth-panel auth-panel--compact",
            div { class: "auth-panel__intro",
                h1 { class: "auth-panel__title", "{title}" }
                p { class: "panel-copy", "{message}" }
            }
            if let (Some(action_label), Some(onaction)) = (action_label, onaction) {
                button {
                    class: "button button--primary",
                    r#type: "button",
                    onclick: move |_| onaction.call(()),
                    "{action_label}"
                }
            }
        }
    }
}

#[component]
fn ToastViewport(feedback: FeedbackSignals) -> Element {
    let status_message = (feedback.status_message)();
    let error_message = (feedback.error_message)();

    if status_message.is_none() && error_message.is_none() {
        return rsx! {};
    }

    rsx! {
        div { class: "toast-viewport",
            if let Some(message) = status_message {
                ToastNotice {
                    key: "status-{message}",
                    title: "操作完成",
                    message,
                    tone_class: "notice--success",
                    role: "status",
                    live: "polite",
                    clear_signal: feedback.status_message,
                }
            }
            if let Some(message) = error_message {
                ToastNotice {
                    key: "error-{message}",
                    title: "操作失败",
                    message,
                    tone_class: "notice--error",
                    role: "alert",
                    live: "assertive",
                    clear_signal: feedback.error_message,
                }
            }
        }
    }
}

#[component]
fn ToastNotice(
    title: &'static str,
    message: String,
    tone_class: &'static str,
    role: &'static str,
    live: &'static str,
    mut clear_signal: Signal<Option<String>>,
) -> Element {
    rsx! {
        article {
            class: "notice notice--toast {tone_class}",
            role: "{role}",
            "aria-live": "{live}",
            "aria-atomic": "true",
            onanimationend: move |_| clear_signal.set(None),
            div { class: "notice__body",
                strong { "{title}" }
                p { "{message}" }
            }
            button {
                class: "button button--ghost button--compact notice__close",
                r#type: "button",
                onclick: move |_| clear_signal.set(None),
                "关闭"
            }
        }
    }
}

#[component]
fn ConsoleModal(
    label: &'static str,
    size_class: &'static str,
    onclose: EventHandler<()>,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: "console-modal-backdrop",
            role: "presentation",
            onclick: move |_| onclose.call(()),
            div {
                class: "console-modal {size_class}",
                role: "dialog",
                "aria-modal": "true",
                "aria-label": "{label}",
                onclick: move |event| event.stop_propagation(),
                {children}
            }
        }
    }
}
