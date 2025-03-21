pub mod app;
mod model;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    use log::Level;
    console_log::init_with_level(Level::Debug).expect("Couldn't init logging");
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}

#[cfg(feature = "ssr")]
pub mod backend;
