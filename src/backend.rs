//! All the startup code and Axum handlers

use crate::app::App;
use crate::app::shell;
use crate::persistence::load_saved;
use crate::persistence::save_running;
use crate::timers::handle_socket;
use axum::Json;
use axum::extract::Path;
use axum::extract::WebSocketUpgrade;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::http::header;
use axum::response::IntoResponse;
use image::Luma;
use log::{error, info};
use once_cell::sync::Lazy;
use qrcode::QrCode;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use std::io::Cursor;
use std::sync::OnceLock;
use uuid::Uuid;
use web_push::{
    ContentEncoding, IsahcWebPushClient, SubscriptionInfo, VapidSignatureBuilder, WebPushClient,
    WebPushMessageBuilder,
};

pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

    use axum::{
        Router,
        routing::{any, get},
    };
    use axum_server::tls_rustls::RustlsConfig;
    use leptos::prelude::*;
    use leptos_axum::{LeptosRoutes, generate_route_list};
    use log::info;
    env_logger::init();

    NOTIFY_KEY
        .set(
            fs::read_to_string("certs/backend_notification_key.pem")
                .expect("Couldn't read backend_notification_key.pem"),
        )
        .expect("Couldn't set notify key");

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    let app = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .route("/:timer_id/qr/:timer_name", get(qr_code))
        .route("/:timer_id/ws/:device_id", any(websocket_handler))
        .route("/:timer_id/:timer_name/manifest.json", get(manifest))
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    let app = app.into_make_service();
    let handle = axum_server::Handle::new();
    let handle2 = handle.clone();
    tokio::spawn(async { shutdown_signal(handle2).await });
    load_saved()?;

    if addr.port() == 8443 {
        // we want a https server
        let tls_key = fs::read_to_string("certs/tls-key.pem").unwrap();
        let tls_cert = fs::read_to_string("certs/tls-cert.pem").unwrap();
        let config = RustlsConfig::from_pem(tls_cert.into_bytes(), tls_key.into_bytes())
            .await
            .expect("Couldn't make config");

        info!["https server started at {addr}"];
        axum_server::bind_rustls(addr, config)
            .handle(handle)
            .serve(app)
            .await
            .unwrap();
    } else {
        info!("listening on http://{}", &addr);
        axum_server::bind(addr)
            .handle(handle)
            .serve(app)
            .await
            .unwrap();
    }
    Ok(())
}

async fn shutdown_signal(handle: axum_server::Handle) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tokio::spawn(async move {
        info!("Shutting down");
        match save_running() {
            Err(e) => error!("Couldn't save running timers: {e}"),
            Ok(_) => (),
        }
        info!("Shut down");
        handle.graceful_shutdown(Some(std::time::Duration::from_secs(10)));
    });
}

pub async fn websocket_handler(
    Path((timer_id, device_id)): Path<(Uuid, Uuid)>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(async move |socket| handle_socket(timer_id, device_id, socket).await)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Subscription {
    pub endpoint: String,
    pub keys: SubscriptionKeys,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubscriptionKeys {
    pub auth: String,
    pub p256dh: String,
}

static WEB_SEND_CLIENT: Lazy<IsahcWebPushClient> = Lazy::new(|| IsahcWebPushClient::new().unwrap());

pub static NOTIFY_KEY: OnceLock<String> = OnceLock::new();

#[derive(Serialize)]
pub struct Notification<'a> {
    pub title: &'a str,
    pub body: &'a str,
}

// TODO - make these be clickable to get back to the timer
/// Asynchronously send a notification in the background
pub fn send_notification(s: &Subscription, notification: &Notification) -> () {
    let message = serde_json::to_string(&notification).unwrap();

    info!("Sending message {:?} {:?}", s, message);
    let subscription_info = SubscriptionInfo::new(
        s.endpoint.clone(),
        s.keys.p256dh.clone(),
        s.keys.auth.clone(),
    );

    let sig_builder = VapidSignatureBuilder::from_pem(
        NOTIFY_KEY
            .get()
            .expect("notify_key was not initialized")
            .as_bytes(),
        &subscription_info,
    )
    .unwrap();

    let mut builder = WebPushMessageBuilder::new(&subscription_info);

    builder.set_payload(ContentEncoding::Aes128Gcm, message.as_bytes());
    builder.set_vapid_signature(sig_builder.build().unwrap());
    let message = builder.build().unwrap();

    // don't hang around for the network request.
    tokio::spawn(async move {
        match WEB_SEND_CLIENT.send(message).await {
            Ok(x) => {
                info!("Message send success: {:?}", x);
            }
            Err(e) => {
                info!("{:?}", e);
            }
        }
    });
}

pub async fn qr_code(
    Path((timer_id, timer_name)): Path<(Uuid, String)>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Get the Host header
    let host = if let Some(host) = headers.get("Host")
        && let Ok(host_str) = host.to_str()
    {
        host_str.to_string()
    } else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Couldn't get host name").into_response();
    };
    let timer_name = urlencoding::encode(&timer_name);
    let url = format!("https://{host}/{timer_id}/timer/{timer_name}");
    let code = QrCode::new(url).unwrap();
    let image = code.render::<Luma<u8>>().module_dimensions(4, 4).build();
    let mut buf = Cursor::new(Vec::new());
    image.write_to(&mut buf, image::ImageFormat::Png).unwrap();

    // Return as a PNG image response
    ([(header::CONTENT_TYPE, "image/png")], buf.into_inner()).into_response()
}

pub async fn manifest(Path((timer_id, timer_name)): Path<(Uuid, String)>) -> impl IntoResponse {
    let body = json! {
        {
            "name": format!("{timer_name} Poker Timer"),
            "short_name": format!("{timer_name} Poker Timer"),
            "description": "A poker tournament timer than can easily shared between players in a game.",
            "icons": [
            {
              "src": "/logo_192.png",
              "type": "image/png",
              "sizes": "192x192"
            },
            {
              "src": "/logo_512.png",
              "type": "image/png",
              "sizes": "512x512",
              "purpose": "any"
            },
            {
              "src": "/logo_512.png",
              "type": "image/png",
              "sizes": "any",
              "purpose": "any"
            }
          ],
            "display": "standalone",
            "display_override": ["window-control-overlay", "standalone"],
            "theme_color": "#000000",
            "background_color": "#ffffff",
            "dir": "ltr",
            "lang": "en",
            "start_url": format!("/{timer_id}/timer/{timer_name}"),
            "scope": format!("/{timer_id}/"),
            "id": format!("/{timer_id}/"),
          }
    };
    Json(body)
}
