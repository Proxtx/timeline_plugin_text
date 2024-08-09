use {
    crate::{
        api::auth,
        config::Config,
        db::{Database, Event},
        Plugin as _, PluginData,
    },
    chrono::{TimeDelta, Utc},
    futures::StreamExt,
    mongodb::bson::doc,
    rocket::{
        http::{CookieJar, Status},
        post, routes,
        serde::json::Json,
        State,
    },
    serde::{Deserialize, Serialize},
    std::sync::Arc,
    types::{
        api::{APIError, APIResult, CompressedEvent},
        timing::{TimeRange, Timing},
    },
};

pub struct Plugin {
    plugin_data: PluginData,
}

impl crate::Plugin for Plugin {
    async fn new(data: crate::PluginData) -> Self
    where
        Self: Sized,
    {
        Plugin { plugin_data: data }
    }

    fn get_type() -> types::api::AvailablePlugins
    where
        Self: Sized,
    {
        types::api::AvailablePlugins::timeline_plugin_text
    }

    fn get_routes() -> Vec<rocket::Route>
    where
        Self: Sized,
    {
        routes![create_text, delete_text]
    }

    fn get_compressed_events(
        &self,
        query_range: &types::timing::TimeRange,
    ) -> std::pin::Pin<
        Box<
            dyn futures::Future<Output = types::api::APIResult<Vec<types::api::CompressedEvent>>>
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
                    data: Box::new(CompressedTextPluginEvent::Text(CompressedTextEvent {
                        text: t.event,
                        id: t.id,
                    })),
                })
            }

            let mut current = query_range.start;

            while current < query_range.end {
                let new_current = current
                    .checked_add_signed(TimeDelta::try_hours(1).unwrap())
                    .unwrap();
                result.push(CompressedEvent {
                    title: "Text".to_string(),
                    time: Timing::Range(TimeRange {
                        start: current,
                        end: new_current,
                    }),
                    data: Box::new(CompressedTextPluginEvent::UploadText),
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
            crate::error::error(
                database.inner().clone(),
                &e,
                Some(<Plugin as crate::Plugin>::get_type()),
                &config.error_report_url,
            );
            (Status::InternalServerError, Json(Err(e.into())))
        }
    }
}

#[post("/delete", data = "<request>")]
async fn delete_text(
    request: String,
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
                doc! {"id": request},
            ),
            None,
        )
        .await
    {
        Ok(_) => (Status::Ok, Json(Ok(()))),
        Err(e) => {
            crate::error::error(
                database.inner().clone(),
                &e,
                Some(<Plugin as crate::Plugin>::get_type()),
                &config.error_report_url,
            );
            (Status::InternalServerError, Json(Err(e.into())))
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
enum CompressedTextPluginEvent {
    Text(CompressedTextEvent),
    UploadText,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct CompressedTextEvent {
    text: String,
    id: String,
}
