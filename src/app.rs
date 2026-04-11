//! All the front end code

use crate::model::*;
use codee::string::JsonSerdeCodec;
use js_sys::{JSON, Promise};
use lazy_regex::regex;
use leptos::{prelude::*, task::spawn_local};
// https://carloskiki.github.io/icondata/
use leptos_icons::Icon;
use leptos_meta::{Link, MetaTags, Script, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    NavigateOptions, StaticSegment,
    components::{Route, Router, Routes},
    hooks::{use_navigate, use_params, use_query},
    path,
};
use leptos_use::{
    UseWebSocketOptions,
    storage::{UseStorageOptions, use_local_storage_with_options},
    use_interval, use_websocket_with_options,
};
use log::{error, info};
use uuid::Uuid;

use leptos::Params;
use leptos_router::params::Params;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_futures::JsFuture;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

/// A component included on all pages that links back to the github repo where people can submit issues and patches.
#[component]
pub fn About() -> impl IntoView {
    view! {
        <a
            style:float="right"
            target="_blank"
            rel="noopener noreferrer"
            href="https://github.com/jpalmucci/shared-poker-timer"
        >
            About
        </a>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/pokertimer.css" />
        <Script src="/utils.js" />

        // content for this welcome page
        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=HomePage />
                    <Route path=path!("/:timer_id/timer") view=TimerPage />
                    <Route path=path!("/:timer_id/settings") view=SettingsPage />
                </Routes>
            </main>
            <About />
        </Router>
    }
}

/// Timers are stored in local storage in the brower. These never saved on the server.
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct TimerRef {
    id: Uuid,
    name: String,
    #[serde(default)]
    break_name: Option<String>,
}

/// a form component that gets a string value
/// The error state is simply a string that is displayed when
/// the current value is invalid. Validator is a function
/// that is called to validate the input
#[component]
fn TextInput(
    #[prop(optional)] name: Option<String>,
    validator: impl Fn(&str) -> Option<String> + 'static,
    signal: RwSignal<Result<String, String>>,
) -> impl IntoView {
    let name2 = name.clone();

    view! {
        <div class="form_group">
            {move || {
                if name.is_some() {
                    Some(view! { <label for=name.clone().unwrap()>{name.clone()}</label> })
                } else {
                    None
                }
            }}
            <input
                type="text"
                class="input-field"
                name=name2
                on:input:target=move |ev| {
                    let v: String = ev.target().value();
                    if let Some(error) = validator(&v) {
                        signal.set(Err(error));
                    } else {
                        signal.set(Ok(v));
                    }
                }
            />
            {move || {
                if let Err(error) = &*signal.read() {
                    Some(view! { <div class="error-message">{error.clone()}</div> })
                } else {
                    None
                }
            }}
        </div>
    }
}

/// a validator that insured there is some value in the string
fn required(s: &str) -> Option<String> {
    if s.len() == 0 {
        Some("Required".to_string())
    } else {
        None
    }
}

/// button that sits at the top right of the screen to bring you to the home page
#[component]
fn CloseButton(href: Option<String>) -> impl IntoView {
    let nav = use_navigate();
    view! {
        <div class="close-button">
            <a on:click=move |_evt| {
                match &href {
                    Some(href) => nav(&href, NavigateOptions::default()),
                    None => nav(&"/", NavigateOptions::default()),
                };
            }>
                <Icon icon=icondata::AiCloseOutlined />
            </a>
        </div>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    use icondata::AiDeleteFilled;
    let (timers, set_timers, _) = use_local_storage_with_options::<Vec<TimerRef>, JsonSerdeCodec>(
        "timers",
        UseStorageOptions::default()
            .delay_during_hydration(true)
            .on_error(|e| {
                error!("localStorage error for 'timers': {e}");
                if let leptos_use::storage::UseStorageError::ItemCodecError(
                    codee::CodecError::Decode(_),
                ) = &e
                {
                    // Clear corrupted data so the app self-heals on next load
                    if let Ok(Some(storage)) = window().local_storage() {
                        let _ = storage.remove_item("timers");
                    }
                }
            }),
    );
    let name_signal = RwSignal::<Result<String, String>>::new(Err("Required".to_string()));
    let break_name_signal = RwSignal::<String>::new(String::new());
    let onsubmit = move |_| {
        let break_name = break_name_signal.get();
        set_timers.write().push(TimerRef {
            id: Uuid::new_v4(),
            name: name_signal.get().unwrap(),
            break_name: if break_name.is_empty() {
                None
            } else {
                Some(break_name)
            },
        });
    };

    let link_signal = RwSignal::<Result<String, String>>::new(Err("Required".to_string()));
    // TODO - make this an environment variable
    let re = regex!(
        r#"^https://([^/]+)/([^/]+)/timer\?name=([^&]*)(?:&break_name=(.*))?$"#
    );

    let validate_link = |s: &str| -> Option<String> {
        if s.len() == 0 {
            Some("Required".to_string())
        } else {
            let cap = re.captures(&s);
            match cap {
                None => Some("Not a valid pokertimer URL".to_string()),
                Some(cap) => match cap.get(2) {
                    None => Some("Not a valid pokertimer URL".to_string()),
                    Some(uuid) => match Uuid::parse_str(uuid.into()) {
                        Ok(_) => None,
                        Err(_) => Some("Not a valid pokertimer URL".to_string()),
                    },
                },
            }
        }
    };
    let import_link = move |_| {
        let link = link_signal.get();
        if let Ok(link) = link {
            if let Some(caps) = re.captures(&link) {
                if let Ok(id) = Uuid::parse_str(caps.get(2).unwrap().as_str()) {
                    let name: String = urlencoding::decode(caps.get(3).unwrap().as_str())
                        .map(|s| s.into_owned())
                        .unwrap_or_else(|_| caps.get(3).unwrap().as_str().to_string());
                    let break_name = caps.get(4).map(|m| {
                        urlencoding::decode(m.as_str())
                            .map(|s| s.into_owned())
                            .unwrap_or_else(|_| m.as_str().to_string())
                    });
                    set_timers.write().push(TimerRef {
                        id,
                        name,
                        break_name,
                    });
                }
            }
        };
    };

    view! {
        <Link rel="manifest" href="/manifest.json" />
        <Title text="Shared Poker timer" />
        <h1>"My Timers"</h1>
        <div class="form">
            <For
                each=move || timers.get()
                key=|timer| timer.id
                children=move |timer| {
                    view! {
                        <p>
                            <a
                                class="links"
                                href=format!(
                                    "/{}/timer?{}",
                                    timer.id,
                                    timer_query(&timer.name, timer.break_name.as_deref()),
                                )
                            >
                                {timer.name.clone()}
                            </a>
                            <a on:click:target=move |_| {
                                set_timers.write().retain(|t| t.id != timer.id);
                            }>
                                <Icon style:float="right" icon=AiDeleteFilled />
                            </a>
                        </p>
                    }
                }
            />
        </div>
        <h2>"Import"</h2>
        <form on:submit=import_link class="form">
            <TextInput
                name="Paste a link here to import a timer".to_string()
                signal=link_signal
                validator=validate_link
            />
            <button disabled=move || link_signal.get().is_err()>"Import"</button>
        </form>
        <h2>Create</h2>
        <form on:submit=onsubmit class="form">
            <TextInput name="Add a new timer".to_string() signal=name_signal validator=required />
            <div class="form_group">
                <label for="break_name">"Break Name (optional)"</label>
                <input
                    type="text"
                    class="input-field"
                    name="break_name"
                    on:input:target=move |ev| {
                        break_name_signal.set(ev.target().value());
                    }
                    prop:value=move || break_name_signal.get()
                />
            </div>
            <button disabled=move || name_signal.get().is_err()>"Create"</button>
        </form>
    }
}

#[derive(Params, PartialEq, Clone, Debug)]
struct RawTimerPageParams {
    timer_id: Option<Uuid>,
}

#[derive(Params, PartialEq, Clone, Debug)]
struct RawTimerNameQuery {
    name: Option<String>,
    break_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct TimerPageParams {
    timer_id: Uuid,
}

fn extract_params() -> Result<(TimerPageParams, TimerNameQuery), String> {
    let params = use_params::<RawTimerPageParams>();
    let query = use_query::<RawTimerNameQuery>();
    match (params.get(), query.get()) {
        (
            Ok(RawTimerPageParams {
                timer_id: Some(timer_id),
            }),
            Ok(RawTimerNameQuery {
                name: Some(name),
                break_name,
            }),
        ) => Ok((
            TimerPageParams { timer_id },
            TimerNameQuery { name, break_name },
        )),
        _ => Err(format!("Bad Request {:?} {:?}", params.get(), query.get())),
    }
}

fn timer_query(name: &str, break_name: Option<&str>) -> String {
    let mut q = format!("name={}", urlencoding::encode(name));
    if let Some(b) = break_name {
        q.push_str(&format!("&break_name={}", urlencoding::encode(b)));
    }
    q
}

#[component]
fn TimerPage() -> impl IntoView {
    view! {
        <CloseButton href=None />
        {|| {
            match extract_params() {
                Ok((TimerPageParams { timer_id }, TimerNameQuery { name, break_name })) => {
                    let manifest_query = timer_query(&name, break_name.as_deref());
                    view! {
                        <Link
                            rel="manifest"
                            href=format!("/{timer_id}/manifest.json?{manifest_query}")
                        />
                        <Title text=format!("{name} Poker Timer") />
                        <TimerComp
                            timer_id=timer_id
                            timer_name=name
                            break_name=break_name
                        />
                    }
                        .into_any()
                }
                Err(e) => view! { <p>Error: {e}</p> }.into_any(),
            }
        }}
    }
}

#[cfg(not(feature = "ssr"))]
fn maybe_add_timer(timer_id: Uuid, timer_name: &str, break_name: Option<&str>) {
    fn write_timers(storage: &web_sys::Storage, timers: &Vec<TimerRef>) {
        match serde_json::to_string(timers) {
            Ok(json) => {
                if let Err(e) = storage.set_item("timers", &json) {
                    error!("Couldn't store timers: {:?}", e);
                }
            }
            Err(e) => error!("Couldn't serialize timers: {e}"),
        }
    }

    let new_ref = || TimerRef {
        id: timer_id,
        name: timer_name.to_string(),
        break_name: break_name.map(|s| s.to_string()),
    };

    if let Ok(Some(storage)) = window().local_storage() {
        match storage.get_item("timers") {
            Ok(Some(item)) => {
                match serde_json::from_str::<Vec<TimerRef>>(&item) {
                    Ok(mut timers) => {
                        if timers.iter().find(|t| t.id == timer_id).is_none() {
                            // timer is not in data, add it
                            timers.push(new_ref());
                            write_timers(&storage, &timers);
                        }
                    }
                    _ => {
                        // couldn't parse the storage, replace it
                        write_timers(&storage, &vec![new_ref()]);
                    }
                }
            }
            _ => {
                // no string exists yet
                write_timers(&storage, &vec![new_ref()]);
            }
        };
    };
}

#[cfg(feature = "ssr")]
fn maybe_add_timer(_timer_id: Uuid, _timer_name: &str, _break_name: Option<&str>) {
    // this does nothing on the server
}

#[cfg(not(feature = "ssr"))]
fn get_device_id() -> Option<Uuid> {
    let storage = match window().local_storage() {
        Ok(Some(s)) => s,
        Ok(None) => {
            error!("localStorage returned None");
            return None;
        }
        Err(e) => {
            error!("localStorage not available: {:?}", e);
            return None;
        }
    };
    let id = match storage.get_item("deviceid") {
        Ok(Some(item)) => match Uuid::parse_str(&item) {
            Ok(id) => id,
            Err(e) => {
                error!("Couldn't parse device_id '{}': {}", item, e);
                Uuid::new_v4()
            }
        },
        Ok(None) => Uuid::new_v4(),
        Err(e) => {
            error!("Couldn't read device_id: {:?}", e);
            return None;
        }
    };
    if let Err(e) = storage.set_item("deviceid", &id.to_string()) {
        error!("Couldn't persist device_id: {:?}", e);
        return None;
    }
    Some(id)
}

#[cfg(feature = "ssr")]
fn get_device_id() -> Option<Uuid> {
    None
}

#[component]
fn TimerComp(
    timer_id: Uuid,
    timer_name: String,
    break_name: Option<String>,
) -> impl IntoView {
    maybe_add_timer(timer_id, &timer_name, break_name.as_deref());
    let qr_query = timer_query(&timer_name, break_name.as_deref());
    let device_id = get_device_id();
    let ws_path = match device_id {
        Some(id) => format!("/{}/ws/{}", timer_id, id),
        None => format!("/{}/ws", timer_id),
    };
    let settable_state = RwSignal::new(TimerCompState::Loading);
    let socket = use_websocket_with_options::<Command, DeviceMessage, JsonSerdeCodec, _, _>(
        &ws_path,
        UseWebSocketOptions::default()
            .reconnect_limit(leptos_use::ReconnectLimit::Infinite)
            .reconnect_interval(3000) // Reconnect after 3 seconds
            .on_message_raw(|m| {
                info!("On Raw Message {:?}", m);
            })
            .on_error(|e| {
                info!("On Error {:?}", e);
            })
            .on_close(|e| {
                info!("WebSocket closed: {:?}", e);
            }),
    );
    Effect::new(move |_| {
        let message = socket.message.get();
        if let Some(dm) = message {
            match dm {
                DeviceMessage::NewState(timer_comp_state) => {
                    if timer_comp_state != settable_state.get_untracked() {
                        settable_state.set(timer_comp_state);
                    }
                }
                DeviceMessage::Beep => {
                    blink_screen();
                    beep();
                }
            };
        }
    });
    let structures = LocalResource::new(|| structure_names());
    let selected_structure = RwSignal::new("Nightly NLHE".to_string());

    view! {
        {{
            move || {
                match settable_state.get() {
                    TimerCompState::Loading => "Loading...".into_any(),
                    TimerCompState::Error(x) => format!("Error: {x}").into_any(),
                    TimerCompState::NoTournament => {
                        view! {
                            <h1>
                                {
                                    let timer_name = timer_name.clone();
                                    move || { timer_name.clone() }
                                }
                            </h1>

                            <p>
                                <h1>"No tournament running"</h1>
                                <form
                                    class="form"
                                    on:submit=move |ev| {
                                        ev.prevent_default();
                                        spawn_local(async move {
                                            create_tournament(
                                                    timer_id,
                                                    selected_structure.get_untracked(),
                                                )
                                                .await
                                                .unwrap();
                                        });
                                    }
                                >
                                    <div class="form-group">
                                        <label for="structure">Choose a structure:</label>
                                        <select
                                            id="structure"
                                            name="structure"
                                            on:change:target=move |ev| {
                                                let v = ev.target().value();
                                                selected_structure.set(v);
                                            }
                                        >
                                            {move || {
                                                match structures.get() {
                                                    None => Vec::new(),
                                                    Some(x) => {
                                                        match x.as_borrowed() {
                                                            Err(_) => Vec::new(),
                                                            Ok(x) => {
                                                                x.iter()
                                                                    .map(|name| {
                                                                        view! { <option value=name.clone() selected={name == "Nightly NLHE"}>{name.clone()}</option> }
                                                                    })
                                                                    .collect_view()
                                                            }
                                                        }
                                                    }
                                                }
                                            }}

                                        </select>
                                    </div>
                                    <button type="submit">Start</button>
                                </form>
                            </p>
                            <div class="qr-code-section">
                                <img src=format!("/{timer_id}/qr?{qr_query}") />
                            </div>
                        }
                            .into_any()
                    }
                    TimerCompState::Running { state, subscribed } => {
                        let next_display_string = state.next.short_level_string(break_name.as_deref());
                        let cur_display_string = state.cur.make_level_string(break_name.as_deref());
                        let timer_name = timer_name.clone();

                        view! {
                            <SettingsButton
                            timer_id=timer_id
                            timer_name=timer_name.clone()
                            break_name=break_name.clone()
                        />
                            <div class="title">
                                {
                                    let timer_name = timer_name.clone();
                                    move || { timer_name.clone() }
                                }
                            </div>

                            <div class="timer-main-content">
                                <div class="timer-info-section">
                                    <div class="level">
                                        "Level " {state.level} ": " {state.cur.game().to_string()}
                                    </div>
                                    <div class="cur-level">{cur_display_string}</div>
                                    <div class="clock">
                                        <Clock state=state.clock />
                                    </div>
                                    <div class="next-level">"Next Level: " {next_display_string}</div>
                                    <p>
                                    <div><NotificationBox timer_id=timer_id subscribed=subscribed device_id=device_id /></div>
                                    <div><WakeLockBox /></div>
                                    </p>
                                    {match state.clock {
                                        ClockState::Paused { .. } => {
                                            view! {
                                                <button on:click={
                                                    let send = socket.send.clone();
                                                    move |_| send(&Command::Resume)
                                                }>Resume</button>
                                            }
                                                .into_any()
                                        }
                                        ClockState::Running { .. } => {
                                            view! {
                                                <button on:click={
                                                    let send = socket.send.clone();
                                                    move |_| send(&Command::Pause)
                                                }>Pause</button>
                                            }
                                                .into_any()
                                        }
                                    }}
                                </div>
                                <div class="qr-code-section">
                                    <img src=format!("/{timer_id}/qr?{qr_query}") />
                                </div>
                            </div>
                        }
                            .into_any()
                    }
                }
            }
        }}
    }
}

#[component]
fn SettingsButton(
    timer_id: Uuid,
    timer_name: String,
    break_name: Option<String>,
) -> impl IntoView {
    let url = format!(
        "/{}/settings?{}",
        timer_id,
        timer_query(&timer_name, break_name.as_deref())
    );
    let nav = use_navigate();
    view! {
        <div class="settings-button">
            <a on:click=move |_evt| {
                nav(&url, NavigateOptions::default());
            }>
                <Icon icon=icondata::AiSettingFilled />
            </a>
        </div>
    }
}

#[component]
fn Clock(state: ClockState) -> impl IntoView {
    let interval = use_interval(1000);
    view! {
        {move || {
            match state {
                ClockState::Paused { .. } => {}
                ClockState::Running { .. } => {
                    interval.counter.get();
                }
            };
            // if we are running, make sure we keep an eye on the interval

            view! { {format!("{state}")} }
        }}
    }
}

#[server]
pub async fn current_state(
    device_id: Option<Uuid>,
    timer_id: Uuid,
) -> Result<TimerCompState, ServerFnError> {
    use crate::timers::Timer;
    Ok(Timer::get(timer_id).to_timer_comp_state(&device_id))
}

#[server]
pub async fn create_tournament(
    timer_id: Uuid,
    structure_name: String,
) -> Result<(), ServerFnError> {
    use crate::timers::Timer;
    let mut timer = Timer::get_mut(timer_id);
    if timer.tournament.is_some() {
        return Ok(());
    }
    info!("Creating tournament {timer_id}");
    timer.make_tournament(structure_name)
}

#[component]
fn NotificationBox(timer_id: Uuid, subscribed: bool, device_id: Option<Uuid>) -> impl IntoView {
    let notifications_available =
        LocalResource::new(|| async { pwa_notification_supported().await });
    view! {
        {move || {
            if device_id.is_some() && notifications_available.get().is_some_and(|v| *v) {
                let device_id = device_id.unwrap();
                Some(
                    view! {
                            <input
                                type="checkbox"
                                prop:checked=subscribed
                                on:click:target=move |evt| {
                                    evt.prevent_default();
                                    if evt.target().checked() {
                                        spawn_local(async move {
                                            match start_notifications(device_id, timer_id).await {
                                                Ok(_) => {}
                                                Err(e) => error!("Couldn't start_notification: {:?}", e),
                                            }
                                        });
                                    } else {
                                        spawn_local(async move {
                                            match stop_notifications(device_id, timer_id).await {
                                                Ok(_) => {}
                                                Err(e) => error!("Couldn't start_notification: {:?}", e),
                                            }
                                        })
                                    }
                                }
                            />
                            "Notifications"
                    },
                )
            } else {
                None
            }
        }}
    }
}

#[component]
fn WakeLockBox() -> impl IntoView {
    let wake_lock_enabled = RwSignal::new(isWakeLockEnabled());

    // Update checkbox state when page becomes visible because most browsers
    // drop it when we are not visible
    Effect::new(move |_| {
        use wasm_bindgen::JsCast;
        use wasm_bindgen::closure::Closure;
        use web_sys::{Event, window};

        let callback = Closure::wrap(Box::new(move |_event: Event| {
            if let Some(window) = window() {
                if let Some(document) = window.document() {
                    if document.visibility_state() == web_sys::VisibilityState::Visible {
                        let current_state = isWakeLockEnabled();
                        wake_lock_enabled.set(current_state);
                    }
                }
            }
        }) as Box<dyn FnMut(_)>);

        if let Some(window) = window() {
            if let Some(document) = window.document() {
                let _ = document.add_event_listener_with_callback(
                    "visibilitychange",
                    callback.as_ref().unchecked_ref(),
                );
            }
        }

        callback.forget();
    });

    view! {
            <input
                type="checkbox"
                prop:checked=move || wake_lock_enabled.get()
                on:change:target=move |evt| {
                    let checked = evt.target().checked();
                    if checked {
                        spawn_local(async move {
                            match JsFuture::from(enableWakeLock()).await {
                                Ok(_) => wake_lock_enabled.set(true),
                                Err(e) => {
                                    error!("Couldn't enable wake lock: {:?}", e);
                                    wake_lock_enabled.set(false);
                                }
                            }
                        });
                    } else {
                        spawn_local(async move {
                            match JsFuture::from(disableWakeLock()).await {
                                Ok(_) => wake_lock_enabled.set(false),
                                Err(e) => {
                                    error!("Couldn't disable wake lock: {:?}", e);
                                    wake_lock_enabled.set(true);
                                }
                            }
                        });
                    }
                }
            />
            "Keep Screen Awake"
    }
}

async fn pwa_notification_supported() -> bool {
    let result = JsFuture::from(notificationsSupported()).await;
    match result {
        Ok(v) => v.as_bool().unwrap(),
        Err(e) => {
            error!("Couldn't determine notification support: {:?}", e);
            false
        }
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen]
    fn notificationsSupported() -> Promise;
    #[wasm_bindgen]
    fn startNotifications() -> Promise;
    #[wasm_bindgen]
    fn stopNotifications() -> Promise;
    #[wasm_bindgen]
    fn enableWakeLock() -> Promise;
    #[wasm_bindgen]
    fn disableWakeLock() -> Promise;
    #[wasm_bindgen]
    fn isWakeLockEnabled() -> bool;
}

async fn start_notifications(device_id: Uuid, timer_id: Uuid) -> Result<(), ServerFnError> {
    let result = JsFuture::from(startNotifications()).await;
    match result {
        Err(e) => Err(ServerFnError::new(format!("start_notifications: {:?}", e))),
        Ok(v) => match JSON::stringify(&v) {
            Err(e) => Err(ServerFnError::new(format!("start_notifications2: {:?}", e))),
            Ok(s) => add_subscription(device_id, timer_id, s.into()).await,
        },
    }
}

#[server]
pub async fn add_subscription(
    device_id: Uuid,
    timer_id: Uuid,
    subscription: String,
) -> Result<(), ServerFnError> {
    use crate::backend::Subscription;
    use crate::timers::Timer;
    let subscription = serde_json::from_str::<Subscription>(&subscription)?;
    let mut t = Timer::get_mut(timer_id);
    info!("{device_id} subscription: {subscription:?}");
    t.subscribe(device_id, subscription);
    Ok(())
}

async fn stop_notifications(device_id: Uuid, timer_id: Uuid) -> Result<(), ServerFnError> {
    match JsFuture::from(stopNotifications()).await {
        Err(e) => Err(ServerFnError::new(format!("{:?}", e))),
        Ok(_) => remove_subscription(device_id, timer_id).await,
    }
}

#[server]
pub async fn remove_subscription(device_id: Uuid, timer_id: Uuid) -> Result<(), ServerFnError> {
    use crate::timers::Timer;
    let mut t = Timer::get_mut(timer_id);
    t.unsubscribe(device_id);
    Ok(())
}

pub fn beep() {
    use js_sys::eval;
    let result = eval(&format!(
        "
    new Audio('/beep.mp3').play();
    ",
    ));
    if let Err(e) = result {
        error!("{e:?}");
    };
}

pub fn blink_screen() {
    use js_sys::eval;
    let result = eval(&format!(
        "
    (function() {{
        const originalBg = document.body.style.backgroundColor;
        const originalFilter = document.body.style.filter;
        const blinkCount = 8;
        let i = 0;

        function doBlink() {{
            if (i >= blinkCount * 2) return;

            if (i % 2 === 0) {{
                document.body.style.backgroundColor = 'white';
                document.body.style.filter = 'brightness(2)';
            }} else {{
                document.body.style.backgroundColor = originalBg;
                document.body.style.filter = originalFilter;
            }}
            i++;
            setTimeout(doBlink, 150);
        }}

        doBlink();
    }})();
    ",
    ));
    if let Err(e) = result {
        error!("{e:?}");
    };
}

#[component]
fn InputOptionalDuration(
    name: String,
    signal: RwSignal<Result<Option<Duration>, String>>,
) -> impl IntoView {
    view! {
        <div class="form-group">
            <label for=name.clone()>{name.clone()}</label>
            " "
            <input
                type="text"
                name=name
                on:input:target=move |evt| {
                    let v = evt.target().value();
                    if v.len() == 0 {
                        signal.set(Ok(None));
                    } else {
                        match v.parse::<i64>() {
                            Ok(v) => signal.set(Ok(Some(Duration::minutes(v)))),
                            Err(e) => signal.set(Err(e.to_string())),
                        }
                    }
                }
                prop:value=move || {
                    match signal.get() {
                        Ok(None) => "".to_string(),
                        Ok(Some(duration)) => duration.num_minutes().to_string(),
                        Err(_) => "".to_string(),
                    }
                }
            />
            " min"
            {move || {
                if let Err(s) = signal.get() {
                    Some(view! { <div class="error-message">{s}</div> })
                } else {
                    None
                }
            }}
        </div>
    }
}

// TODO - change current level's time

#[component]
fn SettingsPage() -> impl IntoView {
    let duration_override_signal =
        RwSignal::<Result<Option<Duration>, String>>::new(Err("Required".to_string()));

    let old_settings: Resource<Result<Option<Duration>, ServerFnError>> = Resource::new(
        || extract_params(),
        |params| async move {
            if let Ok((TimerPageParams { timer_id }, _)) = params {
                tournament_settings(timer_id).await
            } else {
                Err(ServerFnError::new("tournament_settings failed"))
            }
        },
    );
    Effect::new(move || {
        if let Some(Ok(duration_override)) = old_settings.get() {
            duration_override_signal.set(Ok(duration_override));
        }
    });
    let device_id = get_device_id();

    view! {
        {move || {
            match extract_params() {
                Ok((
                    TimerPageParams { timer_id },
                    TimerNameQuery { name: timer_name, break_name },
                )) => {
                    let timer_url_query = timer_query(&timer_name, break_name.as_deref());
                    let execute_command = {
                        let timer_url_query = timer_url_query.clone();
                        move |cmd| {
                            let timer_url_query = timer_url_query.clone();
                            spawn_local(async move {
                                if let Ok(_) = execute_command(cmd, timer_id, device_id).await {
                                    use_navigate()(
                                        &format!("/{}/timer?{}", timer_id, timer_url_query),
                                        NavigateOptions::default(),
                                    );
                                }
                            });
                        }
                    };
                    let error = duration_override_signal.get().is_err();

                    view! {
                        <CloseButton href=Some(format!("/{timer_id}/timer?{timer_url_query}")) />
                        <h1>"Settings"</h1>
                        <form
                            class="form"
                            on:submit:target=move |evt| {
                                evt.prevent_default();
                                if let Ok(v) = duration_override_signal.get() {
                                    let timer_url_query = timer_url_query.clone();
                                    spawn_local(async move {
                                        if let Err(e) = set_tournament_settings(timer_id, v).await {
                                            duration_override_signal.set(Err(e.to_string()));
                                        } else {
                                            let nav = use_navigate();
                                            nav(
                                                &format!("/{timer_id}/timer?{timer_url_query}"),
                                                NavigateOptions::default(),
                                            );
                                        }
                                    })
                                }
                            }
                        >
                            <InputOptionalDuration
                                name="Duration Override".to_string()
                                signal=duration_override_signal
                            />
                            <button type="submit" disabled=error>
                                "Save"
                            </button>
                        </form>
                        <p>
                            <p>
                                <button on:click={
                                    let execute_command = execute_command.clone();
                                    move |_evt| {
                                        execute_command(Command::PrevLevel);
                                    }
                                }>"Previous Level"</button>
                                <button on:click={
                                    let execute_command = execute_command.clone();
                                    move |_evt| {
                                        execute_command(Command::NextLevel);
                                    }
                                }>"Next Level"</button>
                            </p>
                        </p>
                        <p>
                            <p>
                                <button
                                    style:color="red"
                                    on:click=move |_evt| {
                                        execute_command(Command::Terminate);
                                    }
                                >
                                    "TERMINATE"
                                </button>
                            </p>
                        </p>
                    }
                        .into_any()
                }
                Err(e) => {

                    view! { <p>Error: {e}</p> }
                        .into_any()
                }
            }
        }}
    }
}

#[server]
async fn set_tournament_settings(
    timer_id: Uuid,
    duration_override: Option<Duration>,
) -> Result<(), ServerFnError> {
    use crate::timers::Timer;
    Timer::get_mut(timer_id).update_settings(duration_override);
    Ok(())
}

#[server]
async fn tournament_settings(timer_id: Uuid) -> Result<Option<Duration>, ServerFnError> {
    use crate::timers::Timer;
    match &Timer::get(timer_id).tournament {
        Some(t) => Ok(t.duration_override),
        None => Err(ServerFnError::new("running tournament")),
    }
}

#[server]
async fn execute_command(
    cmd: Command,
    timer_id: Uuid,
    device_id: Option<Uuid>,
) -> Result<(), ServerFnError> {
    use crate::timers::Timer;
    Timer::get_mut(timer_id).execute(&cmd, device_id);
    Ok(())
}

#[server]
async fn structure_names() -> Result<Vec<String>, ServerFnError> {
    Ok(crate::structures::STRUCTURES
        .keys()
        .map(|x| x.clone())
        .collect())
}
