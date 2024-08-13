use {
    leptos::{html::Textarea, spawn_local, view, window, IntoView, View},
    serde::{Deserialize, Serialize}, types::timing::Timing
};

pub struct Plugin {
    
}

impl crate::plugin_manager::Plugin for Plugin {
    fn get_style(&self) -> crate::plugin_manager::Style {
        crate::plugin_manager::Style::Acc1
    }

    async fn new(_data: crate::plugin_manager::PluginData) -> Self
        where
            Self: Sized {
        Plugin {}
    }

    fn get_component(&self, data: crate::plugin_manager::PluginEventData) -> crate::plugin_manager::EventResult<Box<dyn FnOnce() -> leptos::View>> {
        let data = data.get_data::<CompressedTextPluginEvent>()?;
        Ok(match data {
            CompressedTextPluginEvent::Text(data) => {
                create_text_event(data)
            },
            CompressedTextPluginEvent::UploadText(timing) => {
                create_upload_text_event(timing)
            }
        })
    }
}

fn create_text_event(compressed_event: CompressedTextEvent) -> Box<dyn FnOnce() -> leptos::View> {
    Box::new(
            move || -> View {
                view! {
                    <div style="display: flex; flex-direction: column; width: 100%; align-items: end; box-sizing: border-box;">
                        <div style="box-sizing: border-box; color: var(--darkColor); padding: calc(var(--contentSpacing) * 0.5); background-color: var(--lightColor); width: 100%; font-family: serif; font-size: 120%;">
                            {move || { compressed_event.text.clone() }}
                        </div>
                        <button
                            on:click=move |_| {
                                let id = compressed_event.id.clone();
                                spawn_local(async move {
                                    match crate::api::api_request(
                                            "/plugin/timeline_plugin_text/delete",
                                            &id,
                                        )
                                        .await
                                    {
                                        Ok(()) => {
                                            let _ = window().location().reload();
                                        }
                                        Err(e) => {
                                            let _ = window()
                                                .alert_with_message(
                                                    &format!("Unable to delete Text: {}", e),
                                                );
                                        }
                                    }
                                })
                            }

                            style="border: none; display: flex; align-items: center; justify-content: center; padding: calc(var(--contentSpacing) * 0.5); padding-left: calc(var(--contentSpacing) * 3); padding-right: calc(var(--contentSpacing) * 3); background-color: var(--accentColor1);"
                        >
                            <svg
                                width="16"
                                height="18"
                                viewBox="0 0 16 18"
                                fill="none"
                                xmlns="http://www.w3.org/2000/svg"
                            >
                                <path
                                    d="M3 18C2.45 18 1.97917 17.8042 1.5875 17.4125C1.19583 17.0208 1 16.55 1 16V3H0V1H5V0H11V1H16V3H15V16C15 16.55 14.8042 17.0208 14.4125 17.4125C14.0208 17.8042 13.55 18 13 18H3ZM13 3H3V16H13V3ZM5 14H7V5H5V14ZM9 14H11V5H9V14Z"
                                    fill="white"
                                ></path>
                            </svg>
                        </button>
                    </div>
                }.into_view()
            }
        )
}

fn create_upload_text_event(timing: Timing) -> Box<dyn FnOnce() -> leptos::View> {
    Box::new(
            move || -> View {
                #[derive(Serialize, Clone, Debug)]
                struct CreateTextRequest {
                    text: String,
                    timing: Timing,
                }

                let textarea_ref = leptos::create_node_ref::<Textarea>();
                view! {
                    <style>
                        ".timeline_plugin_text_textarea {
                            box-sizing: border-box; 
                            border: none; 
                            color: var(--darkColor); 
                            padding: calc(var(--contentSpacing) * 0.5); 
                            background-color: var(--lightColor); 
                            width: 100%; height: 100px; 
                            font-family: serif; 
                            font-size: 120%;
                            resize: vertical;
                        }
                        .timeline_plugin_text_textarea:focus {
                            outline: none;
                        }
                        "
                    </style>
                    <div style="display: flex; flex-direction: column; width: 100%; align-items: end; box-sizing: border-box;">
                        <textarea ref=textarea_ref class="timeline_plugin_text_textarea"></textarea>
                        <button
                            on:click=move |_| {
                                let timing = timing.clone();
                                spawn_local(async move {
                                    match crate::api::api_request(
                                            "/plugin/timeline_plugin_text/create",
                                            &CreateTextRequest {
                                                text: textarea_ref.get().unwrap().value(),
                                                timing,
                                            },
                                        )
                                        .await
                                    {
                                        Ok(()) => {
                                            let _ = window().location().reload();
                                        }
                                        Err(e) => {
                                            let _ = window()
                                                .alert_with_message(
                                                    &format!("Unable to upload Text: {}", e),
                                                );
                                        }
                                    }
                                })
                            }

                            style="border: none; display: flex; align-items: center; justify-content: center; padding: calc(var(--contentSpacing) * 0.5); padding-left: calc(var(--contentSpacing) * 3); padding-right: calc(var(--contentSpacing) * 3); background-color: var(--accentColor1);"
                        >
                            <svg
                                width="19"
                                height="16"
                                viewBox="0 0 19 16"
                                fill="none"
                                xmlns="http://www.w3.org/2000/svg"
                            >
                                <path
                                    d="M0 16V0L19 8L0 16ZM2 13L13.85 8L2 3V6.5L8 8L2 9.5V13Z"
                                    fill="white"
                                ></path>
                            </svg>
                        </button>
                    </div>
                }.into_view()
            }
        )
}

#[derive(Deserialize, Clone, Debug)]
enum CompressedTextPluginEvent {
    Text(CompressedTextEvent),
    UploadText(Timing),
}

#[derive(Deserialize, Clone, Debug)]
struct CompressedTextEvent {
    text: String,
    id: String,
}
