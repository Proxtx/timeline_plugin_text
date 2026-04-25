//! Text plugin: lets the user pin a free-form text snippet to any hour
//! of any day. Stores them in SQLite. Each hour the user hasn't filled
//! in is exposed as an `UploadText` placeholder so the frontend can
//! render an inline editor.

use chrono::{TimeDelta, Utc};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{post, routes, Route, State};
use serde::{Deserialize, Serialize};

use timeline_plugin_sdk::auth::AuthedClient;
use timeline_plugin_sdk::launch::PluginState;
use timeline_plugin_sdk::{
    APIError, APIResult, CompressedEvent, Context, Manifest, Plugin, Style, StoredEvent,
    TimeRange, Timing,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CompressedTextPluginEvent {
    Text { text: String, id: String },
    UploadText { timing: Timing },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredText {
    text: String,
}

pub struct TextPlugin {
    ctx: Context,
}

impl Plugin for TextPlugin {
    async fn new(ctx: Context) -> anyhow::Result<Self> {
        Ok(TextPlugin { ctx })
    }

    fn manifest(&self) -> Manifest {
        Manifest {
            name: self.ctx.config.name.clone(),
            display_name: self
                .ctx
                .config
                .display_name
                .clone()
                .unwrap_or_else(|| "Text".into()),
            style: Style::Acc1,
            icon: None,
            web_entry: Some("timeline_plugin_text_client.js".into()),
        }
    }

    async fn events(&self, range: TimeRange) -> APIResult<Vec<CompressedEvent>> {
        let stored = self
            .ctx
            .db
            .query_range_typed::<StoredText>(&range)
            .await
            .map_err(|e| APIError::DatabaseError(e.to_string()))?;
        let mut out = Vec::with_capacity(stored.len());
        for ev in stored {
            let payload = CompressedTextPluginEvent::Text {
                text: ev.data.text.clone(),
                id: ev.id.clone(),
            };
            out.push(CompressedEvent {
                title: "Text".to_string(),
                time: ev.time,
                data: serde_json::to_value(payload)?,
            });
        }

        // Synthesize one UploadText placeholder per hour in the range.
        let mut current = range.start;
        let hour = TimeDelta::try_hours(1).unwrap_or_default();
        while current < range.end {
            let next = current.checked_add_signed(hour).unwrap_or(range.end);
            let timing = Timing::Range(TimeRange {
                start: current,
                end: next,
            });
            let payload = CompressedTextPluginEvent::UploadText {
                timing: timing.clone(),
            };
            out.push(CompressedEvent {
                title: "Write Text".to_string(),
                time: timing,
                data: serde_json::to_value(payload)?,
            });
            current = next;
        }
        Ok(out)
    }

    fn routes(&self) -> Vec<Route> {
        routes![create_text, delete_text]
    }
}

#[derive(Debug, Clone, Deserialize)]
struct CreateTextRequest {
    text: String,
    timing: Timing,
}

#[post("/create", data = "<request>")]
async fn create_text(
    _auth: AuthedClient,
    request: Json<CreateTextRequest>,
    state: &State<PluginState>,
) -> (Status, Json<APIResult<()>>) {
    let req = request.into_inner();
    let id = Utc::now().timestamp_millis().to_string();
    let stored = match serde_json::to_value(StoredText { text: req.text.clone() }) {
        Ok(v) => v,
        Err(e) => {
            return (
                Status::InternalServerError,
                Json(Err(APIError::SerdeJsonError(e.to_string()))),
            )
        }
    };
    let event = StoredEvent {
        id,
        title: "Text".to_string(),
        time: req.timing,
        data: stored,
    };
    match state.db.upsert(&event).await {
        Ok(()) => (Status::Ok, Json(Ok(()))),
        Err(e) => {
            state.errors.report(format!("create text: {}", e));
            (
                Status::InternalServerError,
                Json(Err(APIError::DatabaseError(e.to_string()))),
            )
        }
    }
}

#[post("/delete", data = "<request>")]
async fn delete_text(
    _auth: AuthedClient,
    request: Json<String>,
    state: &State<PluginState>,
) -> (Status, Json<APIResult<()>>) {
    let id = request.into_inner();
    match state.db.delete(&id).await {
        Ok(()) => (Status::Ok, Json(Ok(()))),
        Err(e) => {
            state.errors.report(format!("delete text: {}", e));
            (
                Status::InternalServerError,
                Json(Err(APIError::DatabaseError(e.to_string()))),
            )
        }
    }
}
