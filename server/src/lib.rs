use {
    rocket::{
        http::{CookieJar, Status},
        post, routes,
        serde::json::Json,
        State,
    }, serde::{Deserialize, Serialize}, server_api::{
        config::Config, db::{Database, Event}, external::{futures::{self, StreamExt}, types::{api::{APIError, APIResult, CompressedEvent}, available_plugins::AvailablePlugins, external::{chrono::{TimeDelta, Utc}, mongodb::bson::doc, serde_json}, timing::{TimeRange, Timing}}}, plugin::{PluginData, PluginTrait}, web::auth
    }, std::sync::Arc
};

pub struct Plugin {
    plugin_data: PluginData,
}

impl PluginTrait for Plugin {
    async fn new(data: crate::PluginData) -> Self
    where
        Self: Sized,
    {
        Plugin { plugin_data: data }
    }

    fn get_type() -> AvailablePlugins
    where
        Self: Sized,
    {
        AvailablePlugins::timeline_plugin_text
    }

    fn get_routes() -> Vec<rocket::Route>
    where
        Self: Sized,
    {
        routes![create_text, delete_text]
    }

    fn get_compressed_events(
        &self,
        query_range: &TimeRange,
    ) -> std::pin::Pin<
        Box<
            dyn futures::Future<Output = APIResult<Vec<CompressedEvent>>>
                + Send,
        >,
    > {
        let filter = Database::generate_range_filter(query_range);
        let plg_filter = Database::generate_find_plugin_filter(Plugin::get_type());
        let filter = Database::combine_documents(filter, plg_filter);
        let database = self.plugin_data.database.clone();
        let query_range = query_range.clone();
        Box::pin(async move {
            let mut cursor = database.get_events::<String>().find(filter, None).await?;
            let mut result = Vec::new();
            while let Some(v) = cursor.next().await {
                let t = v?;
                result.push(CompressedEvent {
                    title: "Text".to_string(),
                    time: t.timing,
                    data: serde_json::to_value(CompressedTextPluginEvent::Text(CompressedTextEvent {
                        text: t.event,
                        id: t.id,
                    })).unwrap(),
                })
            }

            let mut current = query_range.start;

            while current < query_range.end {
                let new_current = current
                    .checked_add_signed(TimeDelta::try_hours(1).unwrap())
                    .unwrap();
                let timing = Timing::Range(TimeRange {
                        start: current,
                        end: new_current,
                });
                result.push(CompressedEvent {
                    title: "Write Text".to_string(),
                    time: timing.clone(),
                    data: serde_json::to_value(CompressedTextPluginEvent::UploadText(timing)).unwrap(),
                });
                current = new_current;
            }

            Ok(result)
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct CreateTextRequest {
    text: String,
    timing: Timing,
}

#[post("/create", data = "<request>")]
async fn create_text(
    request: Json<CreateTextRequest>,
    cookies: &CookieJar<'_>,
    config: &State<Config>,
    database: &State<Arc<Database>>,
) -> (Status, Json<APIResult<()>>) {
    if auth(cookies, config).is_err() {
        return (
            Status::Unauthorized,
            Json(Err(APIError::AuthenticationError)),
        );
    }

    match database
        .register_single_event(&Event {
            timing: request.timing.clone(),
            id: Utc::now().timestamp_millis().to_string(),
            plugin: Plugin::get_type(),
            event: request.text.clone(),
        })
        .await
    {
        Ok(_) => (Status::Ok, Json(Ok(()))),
        Err(e) => {
            server_api::error::error(
                database.inner().clone(),
                &e,
                Some(<Plugin as PluginTrait>::get_type()),
                &config.error_report_url,
            );
            (Status::InternalServerError, Json(Err(e.into())))
        }
    }
}

#[post("/delete", data = "<request>")]
async fn delete_text(
    request: Json<String>,
    cookies: &CookieJar<'_>,
    config: &State<Config>,
    database: &State<Arc<Database>>,
) -> (Status, Json<APIResult<()>>) {
    if auth(cookies, config).is_err() {
        return (
            Status::Unauthorized,
            Json(Err(APIError::AuthenticationError)),
        );
    }

    match database
        .events_collection::<String>()
        .delete_one(
            Database::combine_documents(
                Database::generate_find_plugin_filter(Plugin::get_type()),
                doc! {"id": (*request).clone()},
            ),
            None,
        )
        .await
    {
        Ok(_) => (Status::Ok, Json(Ok(()))),
        Err(e) => {
            server_api::error::error(
                database.inner().clone(),
                &e,
                Some(<Plugin as PluginTrait>::get_type()),
                &config.error_report_url,
            );
            (Status::InternalServerError, Json(Err(e.into())))
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
enum CompressedTextPluginEvent {
    Text(CompressedTextEvent),
    UploadText(Timing),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct CompressedTextEvent {
    text: String,
    id: String,
}
