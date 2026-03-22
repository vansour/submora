use dioxus::prelude::*;

use super::{
    actions,
    state::{FeedbackSignals, PendingState, RefreshState},
};

#[component]
pub fn ControlPlanePanel(
    username: String,
    selected_username: Option<String>,
    mut account_modal_open: Signal<bool>,
    mut links_text: Signal<String>,
    pending: PendingState,
    feedback: FeedbackSignals,
    refresh: RefreshState,
) -> Element {
    let logout_pending = (pending.logout)();
    let selected_label = selected_username.clone();

    rsx! {
        header { class: "panel panel--accent session-panel console-menu",
            div { class: "session-panel__meta",
                div { class: "session-panel__identity",
                    strong { class: "session-panel__user", "{username}" }
                    span { class: "session-panel__role", "管理员" }
                }
                if let Some(selected_label) = selected_label {
                    span { class: "tag session-panel__selection-tag", "{selected_label}" }
                }
            }
            div { class: "button-row session-panel__nav",
                button {
                    class: "button button--danger button--compact",
                    disabled: logout_pending,
                    aria_busy: if logout_pending { "true" } else { "false" },
                    onclick: move |_| actions::logout_session(links_text, pending.logout, feedback, refresh),
                    if logout_pending { "退出中…" } else { "退出" }
                }
                button {
                    class: "button button--ghost button--compact",
                    r#type: "button",
                    onclick: move |_| account_modal_open.set(true),
                    "账户"
                }
            }
        }
    }
}

#[component]
pub fn LoginPanel(
    mut login_username: Signal<String>,
    mut login_password: Signal<String>,
    pending: PendingState,
    feedback: FeedbackSignals,
    refresh: RefreshState,
) -> Element {
    let login_pending = (pending.login)();

    rsx! {
        article { class: "panel panel--hero auth-panel auth-panel--compact",
            div { class: "auth-panel__topline",
                p { class: "eyebrow", "{submora_core::APP_NAME}" }
                span { class: "tag tag--cool", "控制台" }
            }
            div { class: "auth-panel__intro",
                h1 { class: "auth-panel__title", "管理台登录" }
                p { class: "panel-copy", "进入后可直接维护订阅组、源链接和公共聚合入口。" }
            }
            form {
                class: "form-stack auth-form",
                onsubmit: move |event| {
                    event.prevent_default();
                    actions::submit_login(
                        login_username(),
                        login_password(),
                        login_password,
                        pending.login,
                        feedback,
                        refresh,
                    );
                },
                label { class: "field",
                    span { "用户名" }
                    input {
                        autocomplete: "username",
                        disabled: login_pending,
                        value: "{login_username()}",
                        oninput: move |event| login_username.set(event.value()),
                        placeholder: "admin"
                    }
                }
                label { class: "field",
                    span { "密码" }
                    input {
                        autocomplete: "current-password",
                        r#type: "password",
                        disabled: login_pending,
                        value: "{login_password()}",
                        oninput: move |event| login_password.set(event.value()),
                        placeholder: "••••••••"
                    }
                }
                button {
                    class: "button button--primary button--wide",
                    r#type: "submit",
                    disabled: login_pending,
                    aria_busy: if login_pending { "true" } else { "false" },
                    if login_pending { "登录中…" } else { "登录" }
                }
            }
            div { class: "auth-panel__footer",
                div {
                    p { class: "eyebrow", "会话说明" }
                    p { class: "panel-copy", "管理员账户修改后，当前会话会立即失效并要求重新登录。" }
                }
                div { class: "button-row auth-panel__meta-tags",
                    span { class: "tag", "Cookie Session" }
                    span { class: "tag", "CSRF" }
                }
            }
        }
    }
}

#[component]
pub fn AccountPanel(
    mut account_username: Signal<String>,
    mut account_current_password: Signal<String>,
    mut account_new_password: Signal<String>,
    current_username: String,
    onclose: EventHandler<()>,
    pending: PendingState,
    feedback: FeedbackSignals,
    refresh: RefreshState,
) -> Element {
    let current_username_for_submit = current_username.clone();
    let current_username_placeholder = current_username.clone();
    let account_update_pending = (pending.account_update)();

    rsx! {
        article { class: "panel account-panel",
            button {
                class: "button button--ghost button--compact console-modal__close",
                r#type: "button",
                onclick: move |_| onclose.call(()),
                "关闭"
            }
            div { class: "section-head",
                div {
                    h2 { "管理员账户" }
                    p { class: "panel-copy", "修改用户名或密码后，当前会话会立即失效并要求重新登录。" }
                }
                div { class: "button-row",
                    code { "{current_username}" }
                }
            }
            form {
                class: "form-stack",
                onsubmit: move |event| {
                    event.prevent_default();
                    actions::submit_account_update(
                        current_username_for_submit.clone(),
                        account_username(),
                        account_current_password(),
                        account_new_password(),
                        account_username,
                        account_current_password,
                        account_new_password,
                        pending.account_update,
                        feedback,
                        refresh,
                    );
                },
                div { class: "field-grid",
                    label { class: "field",
                        span { "新用户名" }
                        input {
                            disabled: account_update_pending,
                            value: "{account_username()}",
                            oninput: move |event| account_username.set(event.value()),
                            placeholder: current_username_placeholder.clone()
                        }
                    }
                    label { class: "field",
                        span { "当前密码" }
                        input {
                            r#type: "password",
                            disabled: account_update_pending,
                            value: "{account_current_password()}",
                            oninput: move |event| account_current_password.set(event.value()),
                            placeholder: "必填"
                        }
                    }
                }
                label { class: "field",
                    span { "新密码" }
                    input {
                        r#type: "password",
                        disabled: account_update_pending,
                        value: "{account_new_password()}",
                        oninput: move |event| account_new_password.set(event.value()),
                        placeholder: "字母 + 数字 + 符号"
                    }
                }
                button {
                    class: "button button--primary",
                    r#type: "submit",
                    disabled: account_update_pending,
                    aria_busy: if account_update_pending { "true" } else { "false" },
                    if account_update_pending { "更新中…" } else { "更新账户" }
                }
            }
        }
    }
}
