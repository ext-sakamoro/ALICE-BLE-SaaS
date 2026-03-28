// SPDX-License-Identifier: AGPL-3.0-or-later
// ALICE-BLE-SaaS core-engine: BLE protocol platform

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Default)]
struct Stats {
    scans: u64,
    connections: u64,
    gatt_operations: u64,
    pairings: u64,
}

type AppState = Arc<Mutex<Stats>>;

#[derive(Deserialize)]
struct ScanRequest {
    duration_ms: Option<u32>,
    filter_name: Option<String>,
}

#[derive(Deserialize)]
struct ConnectRequest {
    device_address: String,
    timeout_ms: Option<u32>,
}

#[derive(Deserialize)]
struct GattRequest {
    device_address: String,
    service_uuid: String,
    characteristic_uuid: String,
    operation: String,
    value: Option<Vec<u8>>,
}

#[derive(Deserialize)]
struct PairRequest {
    device_address: String,
    passkey: Option<String>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "alice-ble-core",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn scan(
    State(state): State<AppState>,
    Json(req): Json<ScanRequest>,
) -> (StatusCode, Json<Value>) {
    let scan_id = Uuid::new_v4().to_string();
    let duration = req.duration_ms.unwrap_or(5000);
    {
        let mut s = state.lock().unwrap();
        s.scans += 1;
    }
    info!(scan_id = %scan_id, duration_ms = duration, "BLE scan initiated");
    (
        StatusCode::OK,
        Json(json!({
            "scan_id": scan_id,
            "duration_ms": duration,
            "filter": req.filter_name,
            "devices": [
                { "address": "AA:BB:CC:DD:EE:01", "name": "ALICE-Device-01", "rssi": -62, "connectable": true },
                { "address": "AA:BB:CC:DD:EE:02", "name": "ALICE-Device-02", "rssi": -78, "connectable": true },
            ],
        })),
    )
}

async fn connect(
    State(state): State<AppState>,
    Json(req): Json<ConnectRequest>,
) -> (StatusCode, Json<Value>) {
    let connection_id = Uuid::new_v4().to_string();
    let timeout = req.timeout_ms.unwrap_or(10000);
    {
        let mut s = state.lock().unwrap();
        s.connections += 1;
    }
    info!(connection_id = %connection_id, address = %req.device_address, "BLE connection established");
    (
        StatusCode::OK,
        Json(json!({
            "connection_id": connection_id,
            "device_address": req.device_address,
            "timeout_ms": timeout,
            "status": "connected",
            "mtu": 247,
        })),
    )
}

async fn gatt(
    State(state): State<AppState>,
    Json(req): Json<GattRequest>,
) -> (StatusCode, Json<Value>) {
    let op_id = Uuid::new_v4().to_string();
    {
        let mut s = state.lock().unwrap();
        s.gatt_operations += 1;
    }
    info!(op_id = %op_id, operation = %req.operation, "GATT operation executed");
    let response_value = match req.operation.as_str() {
        "read" => vec![0x01u8, 0x02, 0x03],
        "write" => req.value.unwrap_or_default(),
        _ => vec![],
    };
    (
        StatusCode::OK,
        Json(json!({
            "op_id": op_id,
            "device_address": req.device_address,
            "service_uuid": req.service_uuid,
            "characteristic_uuid": req.characteristic_uuid,
            "operation": req.operation,
            "value": response_value,
            "status": "success",
        })),
    )
}

async fn pair(
    State(state): State<AppState>,
    Json(req): Json<PairRequest>,
) -> (StatusCode, Json<Value>) {
    let pair_id = Uuid::new_v4().to_string();
    {
        let mut s = state.lock().unwrap();
        s.pairings += 1;
    }
    info!(pair_id = %pair_id, address = %req.device_address, "BLE pairing completed");
    (
        StatusCode::OK,
        Json(json!({
            "pair_id": pair_id,
            "device_address": req.device_address,
            "method": if req.passkey.is_some() { "passkey" } else { "just_works" },
            "status": "paired",
            "link_key_stored": true,
        })),
    )
}

async fn stats(State(state): State<AppState>) -> Json<Value> {
    let s = state.lock().unwrap();
    Json(json!({
        "scans": s.scans,
        "connections": s.connections,
        "gatt_operations": s.gatt_operations,
        "pairings": s.pairings,
    }))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let state: AppState = Arc::new(Mutex::new(Stats::default()));

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/ble/scan", post(scan))
        .route("/api/v1/ble/connect", post(connect))
        .route("/api/v1/ble/gatt", post(gatt))
        .route("/api/v1/ble/pair", post(pair))
        .route("/api/v1/ble/stats", get(stats))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8138".to_string());
    let addr = format!("0.0.0.0:{}", port);
    info!("alice-ble-core listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
