pub mod model;

#[cfg(feature = "ssr")]
pub mod backend;

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use std::fs;

    use axum::{
        routing::{any, get},
        Router,
    };
    use axum_server::tls_rustls::RustlsConfig;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use log::info;
    use pokertimer::app::*;
    env_logger::init();

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
        .route("/qr/:timer_id/:timer_name", get(backend::qr_code))
        .route("/ws/:timer_id", any(backend::websocket_handler))
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    let app = app.into_make_service();
    if addr.port() == 8443 {
        // we want a https server
        let tls_key = fs::read_to_string("certs/tls-key.pem").unwrap();
        let tls_cert = fs::read_to_string("certs/tls-cert.pem").unwrap();
        let config = RustlsConfig::from_pem(tls_cert.into_bytes(), tls_key.into_bytes())
            .await
            .expect("Couldn't make config");

        info!["https server started at {addr}"];
        let handle = axum_server::Handle::new();
        let handle2 = handle.clone();
        axum_server::bind_rustls(addr, config)
            .handle(handle2)
            .serve(app)
            .await
            .unwrap();
    } else {
        // run our app with hyper
        // `axum::Server` is a re-export of `hyper::Server`
        log!("listening on http://{}", &addr);
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    }
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
