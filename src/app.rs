use crate::model::*;
use chrono::Duration;
use codee::string::JsonSerdeCodec;
use lazy_regex::regex;
use leptos::{logging::error, prelude::*, task::spawn_local};
use leptos_icons::Icon;
use leptos_meta::{provide_meta_context, Link, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    hooks::{use_navigate, use_params},
    path, NavigateOptions, StaticSegment,
};
use leptos_use::{
    storage::{use_local_storage_with_options, UseStorageOptions},
    use_interval, use_websocket_with_options, UseWebSocketOptions,
};
use log::info;
use uuid::Uuid;

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

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/pokertimer.css" />

        // content for this welcome page
        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=HomePage />
                    <Route path=path!("/:timer_id/timer/:timer_name") view=TimerPage />
                    <Route path=path!("/:timer_id/settings/:timer_name") view=SettingsPage />
                </Routes>
            </main>
        </Router>
    }
}

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct TimerRef {
    id: Uuid,
    name: String,
}

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
                        return;
                    }
                    if v.len() == 0 {
                        signal.set(Err("Required".to_string()));
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

fn required(s: &str) -> Option<String> {
    if s.len() == 0 {
        Some("Required".to_string())
    } else {
        None
    }
}

#[component]
fn CloseButton(href: Option<String>) -> impl IntoView {
    let nav = use_navigate();
    view! {
        <div style:color="#FFFFFF" class="close-button">
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
        UseStorageOptions::default().delay_during_hydration(true),
    );
    let name_signal = RwSignal::<Result<String, String>>::new(Err("Required".to_string()));
    let onsubmit = move |_| {
        set_timers.write().push(TimerRef {
            id: Uuid::new_v4(),
            name: name_signal.get().unwrap(),
        });
    };

    let link_signal = RwSignal::<Result<String, String>>::new(Err("Required".to_string()));
    let re = regex!(r#"^https://pokertimer.palmucci.net/timer/([^/]+)/(.*)$"#);

    let validate_link = |s: &str| -> Option<String> {
        if s.len() == 0 {
            Some("Required".to_string())
        } else {
            let cap = re.captures(&s);
            match cap {
                None => Some("Not a valid pokertimer URL".to_string()),
                Some(cap) => match cap.get(1) {
                    None => Some("Not a valid pokertimer URL".to_string()),
                    Some(cap) => match Uuid::parse_str(cap.into()) {
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
                if let Ok(id) = Uuid::parse_str(caps.get(1).unwrap().as_str()) {
                    let name: String = caps.get(2).unwrap().as_str().to_string();
                    set_timers.write().push(TimerRef { id, name });
                }
            }
        };
    };

    view! {
        <Link rel="manifest" href="/manifest.json" />
        <Title text="Shared Poker timer" />
        // TODO: write an explanation of what this is
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
                                    "/{}/timer/{}",
                                    timer.id,
                                    urlencoding::encode(&timer.name),
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
            <button disabled=move || name_signal.get().is_err()>"Create"</button>
        </form>
    }
}

use leptos::Params;
use leptos_router::params::Params;
use web_sys::HtmlInputElement;

#[derive(Params, PartialEq, Clone, Debug)]
struct TimerPageParams {
    timer_id: Option<Uuid>,
    timer_name: Option<String>,
}

fn extract_params() -> Result<(Uuid, String, Uuid), String> {
    let params = use_params::<TimerPageParams>();
    let (device_id, set_device_id, _) = use_local_storage_with_options::<Uuid, JsonSerdeCodec>(
        "deviceid",
        UseStorageOptions::default().delay_during_hydration(true),
    );
    if device_id.get() == Uuid::default() {
        set_device_id.set(Uuid::new_v4());
    }
    match params.get() {
        Ok(TimerPageParams {
            timer_id: Some(timer_id),
            timer_name: Some(timer_name),
        }) => {
            let device_id = device_id.get();
            Ok((timer_id, timer_name, device_id))
        }
        _ => Err(format!("Bad Request {:?}", params.get())),
    }
}

#[component]
fn TimerPage() -> impl IntoView {
    view! {
        <CloseButton href=None />
        {|| {
            match extract_params() {
                Ok((timer_id, timer_name, device_id)) => {
                    view! {
                        <Link
                            rel="manifest"
                            href=format!("/{timer_id}/{timer_name}/manifest.json")
                        />
                        <Title text=format!("{timer_name} Poker Timer") />
                        <TimerComp timer_id=timer_id timer_name=timer_name device_id=device_id />
                    }
                        .into_any()
                }
                Err(e) => view! { <p>Error: {e}</p> }.into_any(),
            }
        }}
    }
}

#[cfg(not(feature = "ssr"))]
fn maybe_add_timer(timer_id: Uuid, timer_name: &str) {
    if let Ok(Some(storage)) = window().local_storage() {
        match storage.get_item("timers") {
            Ok(Some(item)) => {
                match serde_json::from_str::<Vec<TimerRef>>(&item) {
                    Ok(mut timers) => {
                        if let None = timers.iter().find(|t| t.id == timer_id) {
                            // timer is not in data, add it
                            timers.push(TimerRef {
                                id: timer_id,
                                name: timer_name.to_string(),
                            });

                            storage
                                .set_item(
                                    "timers",
                                    &serde_json::to_string::<Vec<TimerRef>>(&timers)
                                        .expect("Couldn't serialize"),
                                )
                                .expect("Couldn't store timers");
                        }
                    }
                    _ => {
                        // couldn't parse the storage, replace it
                        storage
                            .set_item(
                                "timers",
                                &serde_json::to_string::<Vec<TimerRef>>(&vec![TimerRef {
                                    id: timer_id,
                                    name: timer_name.to_string(),
                                }])
                                .expect("Couldn't Serialize"),
                            )
                            .expect("Couldn't store timers");
                    }
                }
            }
            _ => {
                // no string exists yet
                storage
                    .set_item(
                        "timers",
                        &serde_json::to_string::<Vec<TimerRef>>(&vec![TimerRef {
                            id: timer_id,
                            name: timer_name.to_string(),
                        }])
                        .expect("Couldn't Serialize"),
                    )
                    .expect("Couldn't store timers");
            }
        };
    };
}

#[cfg(feature = "ssr")]
fn maybe_add_timer(_timer_id: Uuid, _timer_name: &str) {
    // this does nothing on the server
}

#[component]
fn TimerComp(timer_id: Uuid, timer_name: String, device_id: Uuid) -> impl IntoView {
    maybe_add_timer(timer_id, &timer_name);
    let encoded_name = urlencoding::encode(&timer_name).into_owned();
    let settable_state = RwSignal::new(TimerCompState::Loading);
    let socket = use_websocket_with_options::<Command, DeviceMessage, JsonSerdeCodec, _, _>(
        &format!("/{}/ws/{}", timer_id, device_id),
        UseWebSocketOptions::default()
            .reconnect_limit(leptos_use::ReconnectLimit::Limited(100))
            .on_message_raw(|m| {
                info!("On Raw Message {:?}", m);
            })
            .on_error(|e| {
                info!("On Error {:?}", e);
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
                DeviceMessage::Beep => beep(),
            };
        }
    });

    view! {
        {{
            move || {
                match settable_state.get() {
                    TimerCompState::Loading => "Loading...".into_any(),
                    TimerCompState::Error(x) => format!("Error: {x}").into_any(),
                    TimerCompState::NoTournament => {
                        view! {
                            <p>
                                <div>"No tournament running"</div>
                                <button on:click=move |_| {
                                    spawn_local(async move {
                                        create_tournament(timer_id).await.unwrap();
                                    });
                                }>Start</button>
                            </p>
                            <img src=format!("/{timer_id}/qr/{encoded_name}") />
                        }
                            .into_any()
                    }
                    TimerCompState::Running { state, subscribed } => {
                        let next_display_string = state.next.make_display_string();
                        let cur_display_string = state.cur.make_display_string();
                        let timer_name = timer_name.clone();

                        view! {
                            <div class="level">
                                "Level " {state.level} ": " {state.cur.game().to_string()}
                            </div>
                            <div class="cur-level">{cur_display_string}</div>
                            <div class="clock">
                                <Clock state=state.clock />
                            </div>
                            <div class="next-level">
                                "Next Level: " {state.cur.game().to_string()} " "
                                {next_display_string}
                            </div>
                            <p style:text-align="center">
                                <img src=format!("/{timer_id}/qr/{encoded_name}") />
                            </p>
                            <p>
                                <input
                                    type="checkbox"
                                    checked=subscribed
                                    on:change=move |evt| spawn_local(async move {
                                        let target: HtmlInputElement = event_target(&evt);
                                        if target.checked() {
                                            register_service_worker(device_id, timer_id);
                                        } else {
                                            deregister_service_worker(device_id, timer_id);
                                        }
                                    })
                                />
                                "Notifications"
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
                            <SettingsButton timer_id=timer_id timer_name=timer_name />
                            <p>
                                <button on:click={
                                    let send = socket.send.clone();
                                    move |_| send(&Command::PrevLevel)
                                }>"Previous Level"</button>
                                <button on:click={
                                    let send = socket.send.clone();
                                    move |_| send(&Command::NextLevel)
                                }>"Next Level"</button>
                            </p>
                            <p>
                                <button on:click={
                                    let send = socket.send.clone();
                                    move |_| send(&Command::Terminate)
                                }>"TERMINATE"</button>
                            </p>
                        }
                            .into_any()
                    }
                }
            }
        }}
    }
}

#[component]
fn SettingsButton(timer_id: Uuid, timer_name: String) -> impl IntoView {
    let encoded_name = urlencoding::encode(&timer_name);
    let url = format!("/{}/settings/{}", timer_id, encoded_name);
    let nav = use_navigate();
    view! {
        <button on:click=move |_| {
            nav(&url, NavigateOptions::default());
        }>"Settings"</button>
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
    device_id: Uuid,
    timer_id: Uuid,
) -> Result<TimerCompState, ServerFnError> {
    crate::backend::current_state(device_id, timer_id).await
}

#[server]
pub async fn create_tournament(timer_id: Uuid) -> Result<(), ServerFnError> {
    crate::backend::create_tournament(timer_id).await
}

fn register_service_worker(device_id: Uuid, timer_id: Uuid) {
    use js_sys::eval;
    let result = eval(&format!(
        "if ('serviceWorker' in navigator) {{
            navigator.serviceWorker.register('/service-worker.js')
            .then(registration => {{
                console.log('Service Worker registered:', registration);
                return registration.pushManager.subscribe({{
                    userVisibleOnly: true,
                    applicationServerKey: 'BM7EadIlCgfqJABkpI9L0OsbkyZfL1BnEzjBlYpPAoZt-kDpByG3waoERsCLofkeqRsFBRfbgdJ7ccbSb_oxBf8'
                }});
             }}).then(subscription => {{
                fetch('/{timer_id}/subscribe', {{
                    method: 'POST',
                    body: JSON.stringify({{'device_id': '{device_id}', ...subscription.toJSON() }}),
                    headers: {{ 'Content-Type': 'application/json' }}
                    }});

            }})
            .catch(error => {{
                console.error('Service Worker registration failed:', error);
            }});
        }}"));
    if let Err(e) = result {
        error!("{e:?}");
    };
}

fn deregister_service_worker(device_id: Uuid, timer_id: Uuid) {
    use js_sys::eval;
    let result = eval(&format!(
        "if ('serviceWorker' in navigator) {{
            navigator.serviceWorker.getRegistration().then((reg) =>
            reg.pushManager.getSubscription().then((subscription) => {{
                fetch('/{timer_id}/unsubscribe', {{
                    method: 'POST',
                    body: JSON.stringify({{'device_id': '{device_id}', ...subscription.toJSON() }}),
                    headers: {{ 'Content-Type': 'application/json' }},
                }});
                subscription.unsubscribe();
            }})
            );
       }}
        "
    ));
    if let Err(e) = result {
        error!("{e:?}");
    };
}
pub fn beep() {
    use js_sys::eval;
    let result = eval(&format!(
        "
    new Audio('https://media.geeksforgeeks.org/wp-content/uploads/20190531135120/beep.mp3').play();
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

// TODO - change level time

#[component]
fn SettingsPage() -> impl IntoView {
    let duration_override_signal =
        RwSignal::<Result<Option<Duration>, String>>::new(Err("Required".to_string()));

    let old_settings: Resource<Result<Option<Duration>, ServerFnError>> = Resource::new(
        || extract_params(),
        |params| async move {
            if let Ok((timer_id, _, _)) = params {
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

    view! {
        {move || {
            match extract_params() {
                Ok((timer_id, timer_name, _device_id)) => {
                    let error = duration_override_signal.get().is_err();
                    let encoded_name = urlencoding::encode(&timer_name).into_owned();

                    view! {
                        <CloseButton href=Some(format!("/{timer_id}/timer/{encoded_name}")) />
                        <h1>"Settings"</h1>
                        <form
                            class="form"
                            on:submit:target=move |evt| {
                                evt.prevent_default();
                                if let Ok(v) = duration_override_signal.get() {
                                    let encoded_name = urlencoding::encode(&timer_name)
                                        .into_owned();
                                    spawn_local(async move {
                                        if let Err(e) = set_tournament_settings(timer_id, v).await {
                                            duration_override_signal.set(Err(e.to_string()));
                                        } else {
                                            let nav = use_navigate();
                                            nav(
                                                &format!("/{timer_id}/timer/{encoded_name}"),
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
    crate::backend::set_tournament_settings(timer_id, duration_override)
}

#[server]
async fn tournament_settings(timer_id: Uuid) -> Result<Option<Duration>, ServerFnError> {
    return crate::backend::tourament_settings(timer_id);
}
