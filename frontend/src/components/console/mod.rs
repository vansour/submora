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
    FeedbackSignals, has_unsaved_links, optional_resource_snapshot, resource_snapshot,
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
    let account_username = use_signal(String::new);
    let account_current_password = use_signal(String::new);
    let account_new_password = use_signal(String::new);
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

    let current_user = optional_resource_snapshot(&resources.auth_resource);
    let users = resource_snapshot(&resources.users_resource);
    let current_links_text = links_text();
    let has_unsaved_changes = has_unsaved_links(
        selected_username().as_deref(),
        &current_links_text,
        link_drafts,
    );
    let active_username = selected_username();
    let is_authenticated = current_user.value.is_some();
    let current_username = current_user
        .value
        .clone()
        .map(|user| user.username)
        .unwrap_or_default();

    rsx! {
        AppShell {
            compact: !is_authenticated,
            ToastViewport { feedback }
            if let Some(username) = current_user.value.clone().map(|user| user.username) {
                div { class: "console-frame",
                    ControlPlanePanel {
                        username,
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
                            users: users.value.clone(),
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
            } else {
                LoginPanel {
                    login_username,
                    login_password,
                    pending,
                    feedback,
                    refresh,
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
