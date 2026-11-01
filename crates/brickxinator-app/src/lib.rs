use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use brickxinator_core::{
    ledger::{Id, Operation},
    purchases::PurchaseCommand,
    views::Search,
};
use brickxinator_infra::{attachments, backups, recovery, snapshots, sqlite::Store};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tower::ServiceExt;
use tower_http::services::{ServeDir, ServeFile};

#[derive(Clone)]
pub struct AppState {
    pub instance: Id,
    pub database: PathBuf,
    pub assets: PathBuf,
    pub origin: String,
    pub token: String,
    pub tabs: Arc<Mutex<BTreeMap<Id, Instant>>>,
    pub idle_since: Arc<Mutex<Instant>>,
    pub idle_seconds: u64,
}
impl AppState {
    pub fn new(database: PathBuf, assets: PathBuf, origin: String, idle_seconds: u64) -> Self {
        Self {
            instance: Id::new(),
            database,
            assets,
            origin,
            token: format!("{}{}", Id::new(), Id::new()),
            tabs: Arc::new(Mutex::new(BTreeMap::new())),
            idle_since: Arc::new(Mutex::new(Instant::now())),
            idle_seconds,
        }
    }
    pub fn should_stop(&self) -> bool {
        let mut tabs = self.tabs.lock().expect("tab lock");
        tabs.retain(|_, last| last.elapsed() < Duration::from_secs(30));
        if tabs.is_empty() {
            self.idle_since.lock().expect("idle lock").elapsed()
                >= Duration::from_secs(self.idle_seconds)
        } else {
            *self.idle_since.lock().expect("idle lock") = Instant::now();
            false
        }
    }
}
#[derive(Debug)]
pub struct ApiError(StatusCode, String);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"error":self.1}))).into_response()
    }
}
impl From<Box<dyn std::error::Error + Send + Sync>> for ApiError {
    fn from(error: Box<dyn std::error::Error + Send + Sync>) -> Self {
        let text = error.to_string();
        if text.contains("maintenance") || text.contains("active readers") {
            return Self(
                StatusCode::SERVICE_UNAVAILABLE,
                "Database maintenance is in progress; retry shortly.".into(),
            );
        }
        let message = if text.contains("changed since")
            || text.contains("revision")
            || text.contains("conflict")
            || text.contains("already")
            || text.contains("reconciliation")
        {
            return Self(StatusCode::CONFLICT, text);
        } else if text.contains("does not exist") {
            return Self(StatusCode::NOT_FOUND, text);
        } else if error.downcast_ref::<std::io::Error>().is_some() {
            "File operation failed; check local storage and permissions.".into()
        } else if text.contains("SQL")
            || text.contains("constraint failed")
            || text.contains("no such table")
        {
            "Database validation failed. Check the request or run doctor.".into()
        } else {
            text
        };
        Self(StatusCode::UNPROCESSABLE_ENTITY, message)
    }
}
// Domain execution is synchronous and never blocks Tokio's reactor.
async fn db<F>(state: AppState, work: F) -> Result<Json<Value>, ApiError>
where
    F: FnOnce(&mut Store) -> brickxinator_infra::sqlite::Result<Value> + Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        let mut store = Store::open(&state.database)?;
        work(&mut store)
    })
    .await
    .map_err(|_| {
        ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Local task failed.".into(),
        )
    })?
    .map(|value| Json(exact_json(value)))
    .map_err(Into::into)
}
// JavaScript cannot represent every u64. Preserve large integer read values as
// decimal strings rather than silently rounding owned quantities or paid cents.
fn exact_json(value: Value) -> Value {
    match value {
        Value::Number(ref number) if number.as_u64().is_some_and(|n| n > 9_007_199_254_740_991) => {
            Value::String(number.to_string())
        }
        Value::Array(values) => Value::Array(values.into_iter().map(exact_json).collect()),
        Value::Object(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, exact_json(value)))
                .collect(),
        ),
        value => value,
    }
}
fn decode<T: serde::de::DeserializeOwned>(value: Value) -> brickxinator_infra::sqlite::Result<T> {
    Ok(serde_json::from_value(value)?)
}

pub fn router(state: AppState) -> Router {
    let files = ServeDir::new(&state.assets)
        .not_found_service(ServeFile::new(state.assets.join("index.html")));
    Router::new()
        .route(
            "/api/v1/health",
            get(|State(state):State<AppState>| async move {
                Json(json!({"application":"brickxinator","version":env!("CARGO_PKG_VERSION"),"instance":state.instance}))
            }),
        )
        .route("/api/v1/bootstrap", get(bootstrap))
        .route("/api/v1/state", get(state_view))
        .route("/api/v1/inventory", post(search))
        .route("/api/v1/commands/{kind}", post(command))
        .route(
            "/api/v1/purchases/{id}/correction-preview",
            post(correction_preview),
        )
        .route("/api/v1/costs/{id}", get(costs))
        .route("/api/v1/prices/{id}", get(price_history))
        .route(
            "/api/v1/catalog/{id}/checklist-preview",
            get(definition_checklist),
        )
        .route("/api/v1/sets/{id}", get(set_view))
        .route("/api/v1/sets/{id}/copy-preview", get(copy_preview))
        .route("/api/v1/history/{kind}/{id}", get(history))
        .route(
            "/api/v1/attachments",
            post(upload).layer(DefaultBodyLimit::max(attachments::MAX_BYTES + 65536)),
        )
        .route("/api/v1/attachments/{id}", get(download_attachment))
        .route("/api/v1/snapshot", post(export_snapshot))
        .route("/api/v1/exchange/{kind}",post(export_exchange))
        .route("/api/v1/exchange/{kind}/{id}",get(download_exchange))
        .route("/api/v1/native-import",post(import_exchange).layer(DefaultBodyLimit::max(512*1024*1024)))
        .route(
            "/api/v1/backups",
            get(backup_settings).post(save_backup_settings),
        )
        .route("/api/v1/backups/run", post(run_backups))
        .route("/api/v1/snapshots/{id}", get(download_snapshot))
        .route(
            "/api/v1/restore",
            post(restore_upload).layer(DefaultBodyLimit::max(512 * 1024 * 1024)),
        )
        .route("/api/v1/app-tabs/heartbeat", post(heartbeat))
        .route("/api/v1/app-tabs/disconnect", post(disconnect))
        .fallback_service(files)
        .layer(DefaultBodyLimit::max(5 * 1024 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), security))
        .with_state(state)
}
async fn security(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    let expected = state.origin.strip_prefix("http://").unwrap_or("");
    if host != expected {
        return (StatusCode::FORBIDDEN, "Invalid local host").into_response();
    }
    let origin = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|h| h.to_str().ok());
    if origin.is_some_and(|o| o != state.origin)
        || request
            .headers()
            .get("sec-fetch-site")
            .is_some_and(|h| h == "cross-site")
    {
        return (StatusCode::FORBIDDEN, "Cross-origin access denied").into_response();
    }
    if !matches!(
        *request.method(),
        axum::http::Method::GET | axum::http::Method::HEAD
    ) && (origin != Some(state.origin.as_str())
        || request
            .headers()
            .get("x-brickxinator-token")
            .and_then(|h| h.to_str().ok())
            != Some(state.token.as_str()))
    {
        return (StatusCode::FORBIDDEN, "Invalid local request token/origin").into_response();
    }
    let mut response = next.run(request).await;
    for (name, value) in [
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "no-referrer"),
        ("cache-control", "no-store"),
        (
            "content-security-policy",
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' blob: data:; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",
        ),
    ] {
        response.headers_mut().insert(
            axum::http::HeaderName::from_static(name),
            axum::http::HeaderValue::from_static(value),
        );
    }
    response
}
async fn bootstrap(State(state): State<AppState>) -> Json<Value> {
    Json(json!({"token":state.token,"version":env!("CARGO_PKG_VERSION")}))
}
async fn backup_settings(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let root = state.database.parent().expect("data root").to_owned();
    tokio::task::spawn_blocking(move || -> brickxinator_infra::sqlite::Result<Value> {
        let config = backups::load(&root)?;
        let status = backups::status_view(&root, &config)?;
        Ok(json!({"config":config,"status":status}))
    })
    .await
    .map_err(|_| {
        ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Backup settings unavailable".into(),
        )
    })?
    .map(Json)
    .map_err(Into::into)
}
async fn save_backup_settings(
    State(state): State<AppState>,
    Json(config): Json<backups::Config>,
) -> Result<Json<Value>, ApiError> {
    let root = state.database.parent().expect("data root").to_owned();
    tokio::task::spawn_blocking(move || -> brickxinator_infra::sqlite::Result<Value> {
        let _guard =
            brickxinator_infra::maintenance::Maintenance::acquire(root.join("backup-job"))?;
        backups::save(&root, &config)?;
        Ok(json!({"ok":true}))
    })
    .await
    .map_err(|_| {
        ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Backup settings could not be saved".into(),
        )
    })?
    .map(Json)
    .map_err(Into::into)
}
async fn run_backups(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let root = state.database.parent().expect("data root").to_owned();
    tokio::task::spawn_blocking(move || -> brickxinator_infra::sqlite::Result<Value> {
        Ok(serde_json::to_value(backups::run(
            &root,
            &state.database,
            true,
        )?)?)
    })
    .await
    .map_err(|_| {
        ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Backup task failed".into(),
        )
    })?
    .map(Json)
    .map_err(Into::into)
}
async fn state_view(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let root = state.database.parent().expect("data root").to_owned();
    db(state, move |store| {
        let config=backups::load(&root);
        let (backup_status,backup_notice)=match (config.as_ref().map(|config|backups::status_view(&root,config)).unwrap_or_else(|_|Ok(vec![])),config) {
            (Ok(status),Ok(config))=> {
                let enabled=config.destinations.iter().filter(|d|d.enabled).map(|d|d.id).collect::<Vec<_>>();
                (status.into_iter().filter(|status|status.destination.is_some_and(|id|enabled.contains(&id))).collect::<Vec<_>>(),if enabled.is_empty(){Some("Backups are disabled. Configure destinations and the OS timer in Recovery when ready.")}else{None})
            },
            _=>(vec![],Some("Backup configuration or status is unavailable. Review backup-config.json and backup-status.json in the data directory; inventory remains usable.")),
        };
        let purchases=store.purchases()?;
        let profiles=store.inventory_profiles()?;
        let ledger=store.rebuild()?;
        let copies=ledger.set_copies().values().map(|copy| {
            let source=purchases.iter().find(|purchase|purchase.draft.lines.iter().any(|line|line.id==copy.invoice_line_id));
            let archived=source.is_some_and(|purchase|purchase.archived) || profiles.get(&brickxinator_core::profiles::Target::SetCopy(copy.id)).is_some_and(|p|p.links.archived);
            json!({"id":copy.id,"label":ledger.copy_label(copy.id),"assembly":ledger.assembly(copy.id),"archived":archived})
        }).collect::<Vec<_>>();
        let closed_lots=ledger.classification().iter().filter(|(_,state)|state.complete).map(|(id,_)| {
            let source=ledger.provenance().get(id);
            let label=source.and_then(|line_id|purchases.iter().flat_map(|p|p.draft.lines.iter()).find(|line|line.id==*line_id)).and_then(|line|line.description.as_deref()).unwrap_or("Unlabelled mixed lot");
            json!({"id":id,"label":label,"revision":ledger.holdings()[id].revision})
        }).collect::<Vec<_>>();
        Ok(json!({"dashboard":store.dashboard()?,"settings":store.settings()?.into_values().collect::<Vec<_>>(),"catalog":store.catalog()?.into_values().collect::<Vec<_>>(),"profiles":profiles.into_values().collect::<Vec<_>>(),"purchases":purchases,"attachments":store.attachments()?,"set_copies":copies,"closed_lots":closed_lots,"backup_status":backup_status,"backup_notice":backup_notice}))
    }).await
}
async fn price_history(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    db(state, move |store| {
        Ok(serde_json::to_value(store.price_history(parse_id(&id)?)?)?)
    })
    .await
}
async fn definition_checklist(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    db(state, move |store| {
        Ok(serde_json::to_value(
            store.prepare_definition_expected(parse_id(&id)?)?,
        )?)
    })
    .await
}
async fn search(
    State(state): State<AppState>,
    Json(query): Json<Search>,
) -> Result<Json<Value>, ApiError> {
    db(state, move |store| {
        Ok(serde_json::to_value(store.search_inventory(&query)?)?)
    })
    .await
}
#[derive(Deserialize)]
struct CorrectionInput {
    expected_revision: u64,
    draft: brickxinator_core::purchases::Draft,
}
async fn correction_preview(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<CorrectionInput>,
) -> Result<Json<Value>, ApiError> {
    db(state, move |store| {
        Ok(serde_json::to_value(store.preview_purchase_correction(
            parse_id(&id)?,
            input.expected_revision,
            &input.draft,
        )?)?)
    })
    .await
}
async fn command(
    State(state): State<AppState>,
    Path(kind): Path<String>,
    Json(mut value): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    if let Value::Object(fields) = &mut value {
        fields.insert("actor".into(), Value::String(local_actor()));
    }
    db(state, move |store| {
        Ok(match kind.as_str() {
            "purge" => {
                store.purge_draft(decode(value)?)?;
                json!({"deleted":true})
            }
            "purchases" => {
                serde_json::to_value(store.execute_purchase(decode::<PurchaseCommand>(value)?)?)?
            }
            "settings" => serde_json::to_value(store.execute_settings(decode(value)?)?)?,
            "catalog" => serde_json::to_value(store.execute_catalog(decode(value)?)?)?,
            "profiles" => serde_json::to_value(store.execute_profile(decode(value)?)?)?,
            "verification" => serde_json::to_value(store.execute_verification(decode(value)?)?)?,
            "stock" => serde_json::to_value(store.execute_stock(decode(value)?)?)?,
            "shipping" => serde_json::to_value(store.execute_allocation(decode(value)?)?)?,
            "inventory" => serde_json::to_value(store.commit(decode::<Operation>(value)?)?)?,
            _ => return Err("unknown command family".into()),
        })
    })
    .await
}
fn local_actor() -> String {
    std::env::var("USER")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "local-user".into())
}
fn parse_id(value: &str) -> brickxinator_infra::sqlite::Result<Id> {
    Ok(serde_json::from_value(json!(value))?)
}
async fn costs(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    db(state, move |store| {
        Ok(serde_json::to_value(store.purchase_costs(parse_id(&id)?)?)?)
    })
    .await
}
async fn set_view(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    db(state,move|store|{let id=parse_id(&id)?;Ok(json!({"verification":store.set_verification(id)?,"contents":store.set_contents(id)?.iter().map(|c|json!({"line_id":c.line_id,"holding_id":c.holding_id,"holding":c.holding,"links":c.links})).collect::<Vec<_>>()}))}).await
}
async fn copy_preview(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    db(state, move |store| {
        Ok(serde_json::to_value(
            store.prepare_expected_copy(parse_id(&id)?)?,
        )?)
    })
    .await
}
async fn history(
    State(state): State<AppState>,
    Path((kind, id)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    db(state,move|store| {let id=parse_id(&id)?;Ok(match kind.as_str() {
        "purchase"=>json!(store.purchase_history(id)?.iter().map(|h|json!({"at":h.committed_at,"actor":h.actor,"reason":h.reason,"before":h.before,"after":h.after})).collect::<Vec<_>>()),
        "holding"=>json!(store.holding_history(id)?.iter().map(|h|json!({"at":h.committed_at,"actor":h.actor,"reason":h.reason,"before":h.before,"after":h.after,"classification":h.classification_after})).collect::<Vec<_>>()),
        "set"=>json!(store.verification_history(id)?.iter().map(|h|json!({"at":h.committed_at,"actor":h.actor,"reason":h.reason,"before":h.before,"after":h.after})).collect::<Vec<_>>()),
        _=>return Err("unknown history kind".into()),
    })}).await
}
async fn upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<Value>, ApiError> {
    let mut meta = None;
    let mut bytes = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "Invalid upload".into()))?
    {
        match field.name() {
            Some("meta") => {
                meta = Some(field.text().await.map_err(|_| {
                    ApiError(StatusCode::BAD_REQUEST, "Invalid upload metadata".into())
                })?)
            }
            Some("file") => {
                bytes = Some(field.bytes().await.map_err(|_| {
                    ApiError(StatusCode::BAD_REQUEST, "Invalid uploaded bytes".into())
                })?)
            }
            _ => {}
        }
    }
    let mut command: attachments::Command = serde_json::from_str(
        &meta.ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "Upload metadata missing".into()))?,
    )
    .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "Invalid upload metadata".into()))?;
    command.actor = local_actor();
    let bytes =
        bytes.ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "Upload file missing".into()))?;
    db(state, move |store| {
        Ok(serde_json::to_value(store.attach(command, &bytes)?)?)
    })
    .await
}
async fn download_attachment(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let (record, bytes) = tokio::task::spawn_blocking(move || {
        let store = Store::open(&state.database)?;
        store.attachment_bytes(parse_id(&id)?)
    })
    .await
    .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "Download failed".into()))?
    .map_err(ApiError::from)?;
    let disposition = if record.media_type == "application/pdf" {
        "attachment"
    } else {
        "inline"
    };
    Ok((
        [
            (header::CONTENT_TYPE, record.media_type),
            (
                header::CONTENT_DISPOSITION,
                format!("{disposition}; filename=\"{}\"", record.relative_path),
            ),
        ],
        bytes,
    )
        .into_response())
}
async fn export_snapshot(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    tokio::task::spawn_blocking(move || -> brickxinator_infra::sqlite::Result<Value> {
        let id = Id::new();
        let dir = state
            .database
            .parent()
            .ok_or("missing data directory")?
            .join("exports");
        std::fs::create_dir_all(&dir)?;
        let output = dir.join(format!("{id}.zip"));
        let config = backups::config_path(state.database.parent().ok_or("missing data root")?);
        snapshots::snapshot(
            &state.database,
            &output,
            "manual",
            config.exists().then_some(config.as_path()),
        )?;
        Ok(json!({"id":id,"url":format!("/api/v1/snapshots/{id}")}))
    })
    .await
    .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "Snapshot failed".into()))?
    .map(Json)
    .map_err(Into::into)
}
async fn download_snapshot(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let id = parse_id(&id).map_err(ApiError::from)?;
    let path = state
        .database
        .parent()
        .ok_or_else(|| {
            ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Missing data root".into(),
            )
        })?
        .join("exports")
        .join(format!("{id}.zip"));
    stream_download(path, "application/zip", "brickxinator-snapshot.zip").await
}
async fn stream_download(
    path: PathBuf,
    mime: &'static str,
    name: &'static str,
) -> Result<Response, ApiError> {
    let mut response = ServeFile::new(path)
        .oneshot(Request::new(axum::body::Body::empty()))
        .await
        .expect("ServeFile is infallible")
        .map(axum::body::Body::new);
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static(mime),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        axum::http::HeaderValue::from_str(&format!("attachment; filename={name}"))
            .expect("static filename"),
    );
    Ok(response)
}
async fn uploaded_file(mut multipart: Multipart) -> Result<tempfile::NamedTempFile, ApiError> {
    use tokio::io::AsyncWriteExt;
    let staged = tempfile::NamedTempFile::new().map_err(|error| {
        ApiError::from(Box::new(error) as Box<dyn std::error::Error + Send + Sync>)
    })?;
    let mut output = tokio::fs::File::from_std(staged.reopen().map_err(|error| {
        ApiError::from(Box::new(error) as Box<dyn std::error::Error + Send + Sync>)
    })?);
    let mut found = false;
    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "Invalid archive upload".into()))?
    {
        if field.name() != Some("file") {
            continue;
        }
        if found {
            return Err(ApiError(
                StatusCode::BAD_REQUEST,
                "Upload exactly one archive".into(),
            ));
        }
        found = true;
        while let Some(chunk) = field.chunk().await.map_err(|_| {
            ApiError(
                StatusCode::BAD_REQUEST,
                "Archive exceeds upload limit or is incomplete".into(),
            )
        })? {
            output.write_all(&chunk).await.map_err(|_| {
                ApiError(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Could not stage archive on local storage".into(),
                )
            })?;
        }
    }
    if !found {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "Archive file missing".into(),
        ));
    }
    output.sync_all().await.map_err(|_| {
        ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not finish archive staging".into(),
        )
    })?;
    drop(output);
    Ok(staged)
}
async fn restore_upload(
    State(state): State<AppState>,
    multipart: Multipart,
) -> Result<Json<Value>, ApiError> {
    let file = uploaded_file(multipart).await?;
    tokio::task::spawn_blocking(move || -> brickxinator_infra::sqlite::Result<Value> {
        let safety = state
            .database
            .parent()
            .ok_or("missing data root")?
            .join("backups/safety");
        Ok(serde_json::to_value(recovery::restore(
            &state.database,
            file.path(),
            safety,
        )?)?)
    })
    .await
    .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "Restore failed".into()))?
    .map(Json)
    .map_err(Into::into)
}
async fn export_exchange(
    State(state): State<AppState>,
    Path(kind): Path<String>,
) -> Result<Json<Value>, ApiError> {
    if !matches!(kind.as_str(), "native" | "csv") {
        return Err(ApiError(
            StatusCode::NOT_FOUND,
            "Unknown export format".into(),
        ));
    }
    tokio::task::spawn_blocking(move || -> brickxinator_infra::sqlite::Result<Value> {
        let id = Id::new();
        let root = state
            .database
            .parent()
            .ok_or("missing data root")?
            .join("exports");
        std::fs::create_dir_all(&root)?;
        if kind == "native" {
            brickxinator_infra::exchange::export_archive(
                &state.database,
                root.join(format!("{id}.native.zip")),
            )?;
        } else {
            brickxinator_infra::exchange::inventory_csv(
                &state.database,
                root.join(format!("{id}.csv")),
            )?;
        }
        Ok(json!({"id":id,"url":format!("/api/v1/exchange/{kind}/{id}")}))
    })
    .await
    .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "Export failed".into()))?
    .map(Json)
    .map_err(Into::into)
}
async fn download_exchange(
    State(state): State<AppState>,
    Path((kind, id)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let id = parse_id(&id).map_err(ApiError::from)?;
    let (suffix, mime, name) = match kind.as_str() {
        "native" => ("native.zip", "application/zip", "brickxinator-native.zip"),
        "csv" => (
            "csv",
            "text/csv; charset=utf-8",
            "brickxinator-inventory.csv",
        ),
        _ => {
            return Err(ApiError(
                StatusCode::NOT_FOUND,
                "Unknown export format".into(),
            ));
        }
    };
    let path = state
        .database
        .parent()
        .ok_or_else(|| {
            ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Missing data root".into(),
            )
        })?
        .join("exports")
        .join(format!("{id}.{suffix}"));
    stream_download(path, mime, name).await
}
async fn import_exchange(
    State(state): State<AppState>,
    multipart: Multipart,
) -> Result<Json<Value>, ApiError> {
    let file = uploaded_file(multipart).await?;
    tokio::task::spawn_blocking(move||->brickxinator_infra::sqlite::Result<Value>{
        let safety=state.database.parent().ok_or("missing data root")?.join("backups/safety");
        Ok(json!({"imported":brickxinator_infra::exchange::import_archive(&state.database,file.path(),safety)?}))
    }).await.map_err(|_|ApiError(StatusCode::INTERNAL_SERVER_ERROR,"Import failed".into()))?.map(Json).map_err(Into::into)
}

#[derive(Deserialize, Serialize)]
struct Tab {
    tab_id: Id,
}
async fn heartbeat(State(state): State<AppState>, Json(tab): Json<Tab>) -> Json<Value> {
    state
        .tabs
        .lock()
        .expect("tabs")
        .insert(tab.tab_id, Instant::now());
    *state.idle_since.lock().expect("idle") = Instant::now();
    Json(json!({"ok":true}))
}
async fn disconnect(State(state): State<AppState>, Json(tab): Json<Tab>) -> Json<Value> {
    state.tabs.lock().expect("tabs").remove(&tab.tab_id);
    *state.idle_since.lock().expect("idle") = Instant::now();
    Json(json!({"ok":true}))
}
