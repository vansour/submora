use dioxus::prelude::*;

#[component]
pub fn AppShell(compact: bool, children: Element) -> Element {
    let page_class = if compact {
        "shell-page shell-page--compact"
    } else {
        "shell-page shell-page--app"
    };
    let shell_class = if compact {
        "shell shell--compact"
    } else {
        "shell shell--app"
    };
    let content_class = if compact {
        "content content--compact"
    } else {
        "content"
    };

    rsx! {
        div { class: "{page_class}",
            div { class: "{shell_class}",
                main { class: "{content_class}", {children} }
            }
        }
    }
}
