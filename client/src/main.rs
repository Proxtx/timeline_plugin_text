use leptos::html::Textarea;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};

use timeline_plugin_client_sdk::{plugin_entry, ApiClient, PluginContext, Timing};

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CompressedTextPluginEvent {
    Text { text: String, id: String },
    UploadText { timing: Timing },
}

#[derive(Debug, Clone, Serialize)]
struct CreateTextRequest {
    text: String,
    timing: Timing,
}

fn main() {
    console_error_panic_hook::set_once();
}

fn render(ctx: PluginContext) -> impl IntoView {
    let Ok(payload) = serde_json::from_value::<CompressedTextPluginEvent>(ctx.event.data.clone()) else {
        return view! { <div>Malformed text event</div> }.into_any();
    };
    let api = ApiClient::new(ctx.api_base.clone());
    match payload {
        CompressedTextPluginEvent::Text { text, id } => view_existing(api, text, id),
        CompressedTextPluginEvent::UploadText { timing } => view_upload(api, timing),
    }
}

fn view_existing(api: ApiClient, text: String, id: String) -> AnyView {
    let on_delete = move |_| {
        let api = api.clone();
        let id = id.clone();
        spawn_local(async move {
            match api.post::<_, ()>("/delete", &id).await {
                Ok(()) => reload(),
                Err(e) => alert(&format!("Unable to delete Text: {}", e)),
            }
        });
    };
    view! {
        <div class="text-row">
            <div class="text-body">{text}</div>
            <button class="text-btn" on:click=on_delete>
                <DeleteIcon />
            </button>
        </div>
    }
    .into_any()
}

fn view_upload(api: ApiClient, timing: Timing) -> AnyView {
    let textarea_ref: NodeRef<Textarea> = NodeRef::new();
    let on_send = move |_| {
        let api = api.clone();
        let timing = timing.clone();
        let Some(node) = textarea_ref.get() else { return };
        let text = node.value();
        spawn_local(async move {
            let req = CreateTextRequest { text, timing };
            match api.post::<_, ()>("/create", &req).await {
                Ok(()) => reload(),
                Err(e) => alert(&format!("Unable to upload Text: {}", e)),
            }
        });
    };
    view! {
        <div class="text-row">
            <textarea node_ref=textarea_ref class="text-textarea" />
            <button class="text-btn" on:click=on_send>
                <SendIcon />
            </button>
        </div>
    }
    .into_any()
}

fn reload() {
    if let Some(win) = web_sys::window() {
        let _ = win.location().reload();
    }
}

fn alert(msg: &str) {
    if let Some(win) = web_sys::window() {
        let _ = win.alert_with_message(msg);
    }
}

#[component]
fn DeleteIcon() -> impl IntoView {
    view! {
        <svg width="16" height="18" viewBox="0 0 16 18" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path
                d="M3 18C2.45 18 1.97917 17.8042 1.5875 17.4125C1.19583 17.0208 1 16.55 1 16V3H0V1H5V0H11V1H16V3H15V16C15 16.55 14.8042 17.0208 14.4125 17.4125C14.0208 17.8042 13.55 18 13 18H3ZM13 3H3V16H13V3ZM5 14H7V5H5V14ZM9 14H11V5H9V14Z"
                fill="white"
            />
        </svg>
    }
}

#[component]
fn SendIcon() -> impl IntoView {
    view! {
        <svg width="19" height="16" viewBox="0 0 19 16" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M0 16V0L19 8L0 16ZM2 13L13.85 8L2 3V6.5L8 8L2 9.5V13Z" fill="white" />
        </svg>
    }
}

plugin_entry!(render);
