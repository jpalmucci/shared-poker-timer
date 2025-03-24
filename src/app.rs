use crate::model::*;
use codee::string::JsonSerdeCodec;
use lazy_regex::regex;
use leptos::{logging::error, prelude::*, task::spawn_local};
use leptos_icons::Icon;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    hooks::use_params,
    path, StaticSegment,
};
use leptos_use::{
    core::ConnectionReadyState,
    storage::{use_local_storage_with_options, UseStorageOptions},
    use_interval, use_websocket_with_options, UseWebSocketOptions, UseWebSocketReturn,
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

        // sets the document title
        <Title text="Welcome to Leptos" />

        // content for this welcome page
        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=HomePage />
                    <Route path=path!("/timer/:timer_id/:timer_name") view=TimerPage />
                </Routes>
            </main>
        </Router>
    }
}

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct Timer {
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

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    use icondata::AiDeleteFilled;
    let (timers, set_timers, _) = use_local_storage_with_options::<Vec<Timer>, JsonSerdeCodec>(
        "timers",
        UseStorageOptions::default().delay_during_hydration(true),
    );
    let name_signal = RwSignal::<Result<String, String>>::new(Err("Required".to_string()));
    let onsubmit = move |_| {
        set_timers.write().push(Timer {
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
                    set_timers.write().push(Timer { id, name });
                }
            }
        };
    };

    view! {
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
                                    "/timer/{}/{}",
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
        {|| {
            match extract_params() {
                Ok((timer_id, timer_name, device_id)) => {
                    view! {
                        <TimerComp timer_id=timer_id timer_name=timer_name device_id=device_id />
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

#[derive(serde::Deserialize, serde::Serialize, Clone, PartialEq)]
enum TimerCompState {
    Loading,
    NoTournament,
    Running(DeviceState),
    Error(String),
}

impl TimerCompState {
    fn from_current(x: Result<Option<DeviceState>, ServerFnError>) -> TimerCompState {
        match x {
            Err(e) => TimerCompState::Error(e.to_string()),
            Ok(None) => TimerCompState::NoTournament,
            Ok(Some(ds)) => TimerCompState::Running(ds),
        }
    }
    fn from_create(x: Result<DeviceState, ServerFnError>) -> TimerCompState {
        match x {
            Err(e) => TimerCompState::Error(e.to_string()),
            Ok(ds) => TimerCompState::Running(ds),
        }
    }
}

#[cfg(not(feature = "ssr"))]
fn maybe_add_timer(timer_id: Uuid, timer_name: &str) {
    // add the timer to local storage if we do not have it yet
    let (timers, set_timers, _) = use_local_storage_with_options::<Vec<Timer>, JsonSerdeCodec>(
        "timers",
        UseStorageOptions::default().delay_during_hydration(true),
    );
    let timer_name = timer_name.to_string();
    Effect::new(move |_| {
        if let None = timers.read().iter().find(|t| t.id == timer_id) {
            // timer is not in data, add it
            set_timers.write().push(Timer {
                id: timer_id,
                name: timer_name.clone(),
            });
        }
    });
}

#[cfg(feature = "ssr")]
fn maybe_add_timer(_timer_id: Uuid, _timer_name: &str) {}

#[component]
fn TimerComp(timer_id: Uuid, timer_name: String, device_id: Uuid) -> impl IntoView {
    // TODO - fix this
    // maybe_add_timer(timer_id, &timer_name);
    let initial_state = Resource::new(
        || extract_params(),
        move |params| async move {
            if let Ok((timer_id, _timer_name, device_id)) = params {
                current_state(device_id, timer_id).await
            } else {
                current_state(Uuid::max(), timer_id).await
            }
        },
    );
    let settable_state = RwSignal::new(TimerCompState::Loading);
    let my_state = move || {
        if initial_state.get() == None {
            TimerCompState::Loading
        } else if settable_state.get() != TimerCompState::Loading {
            settable_state.get()
        } else if let Some(x) = initial_state.get() {
            TimerCompState::from_current(x)
        } else {
            TimerCompState::Loading
        }
    };
    let UseWebSocketReturn {
        ready_state,
        message,
        open,
        close,
        ..
    } = use_websocket_with_options::<DeviceMessage, DeviceMessage, JsonSerdeCodec, _, _>(
        &format!("/ws/{timer_id}"),
        UseWebSocketOptions::default()
            .immediate(false)
            .on_message_raw(|m| {
                info!("On Raw Message {:?}", m);
            })
            .on_error(|e| {
                info!("On Error {:?}", e);
            }),
    );
    Effect::new(move |_| match my_state() {
        TimerCompState::Running(device_state) => {
            if ready_state.get() == ConnectionReadyState::Closed {
                info!("Opening channel");
                open();
            } else {
                let message = message.get();
                if let Some(dm) = message {
                    if let TournamentMessage::LevelUp(_) = &dm.msg {
                        beep();
                    }
                    let new_device_state = dm.into_device_state();
                    if new_device_state != device_state {
                        settable_state.set(TimerCompState::Running(new_device_state));
                    }
                }
            }
        }
        _ => {
            if ready_state.get() != ConnectionReadyState::Closed {
                close()
            }
        }
    });

    view! {
        <Suspense fallback=|| {
            "Loading..."
        }>

            {move || {
                match my_state() {
                    TimerCompState::Loading => "Loading...".into_any(),
                    TimerCompState::Error(x) => format!("Error: {x}").into_any(),
                    TimerCompState::NoTournament => {
                        view! {
                            <div>"No tournament running"</div>
                            <button on:click=move |_| {
                                spawn_local(async move {
                                    let t = create_tournament(device_id, timer_id).await;
                                    settable_state.set(TimerCompState::from_create(t))
                                });
                            }>Start</button>
                        }
                            .into_any()
                    }
                    TimerCompState::Running(DeviceState { state, .. }) => {
                        let next_display_string = state.next.make_display_string();
                        let cur_display_string = state.cur.make_display_string();

                        view! {
                            <div class="level">"Level " {state.level}</div>
                            <div class="cur_level">{cur_display_string}</div>
                            <Clock state=state.clock />
                            <div>"Next Level: " {next_display_string}</div>
                        }
                            .into_any()
                    }
                }
            }}
            {move || {
                if let TimerCompState::Running(DeviceState { subscribed, .. }) = my_state() {
                    Some(

                        view! {
                            <div>
                                <input
                                    type="checkbox"
                                    checked=subscribed
                                    on:input=move |evt| spawn_local(async move {
                                        let value = evt.value_of().is_truthy();
                                        if value {
                                            register_service_worker(device_id, timer_id);
                                        } else {
                                            deregister_service_worker(device_id, timer_id);
                                        }
                                    })
                                />
                                "Notifications"
                            </div>
                        },
                    )
                } else {
                    None
                }
            }}
            {move || {
                if let TimerCompState::Running(DeviceState { state, .. }) = my_state() {
                    match state.clock {
                        ClockState::Paused { .. } => {
                            Some(
                                view! {
                                    <button on:click=move |_| spawn_local(async move {
                                        match resume_tournament(device_id, timer_id).await {
                                            Ok(_) => {}
                                            Err(e) => {
                                                settable_state.set(TimerCompState::Error(e.to_string()))
                                            }
                                        }
                                    })>"Resume"</button>
                                }
                                    .into_any(),
                            )
                        }
                        ClockState::Running { .. } => {
                            Some(
                                view! {
                                    <button on:click=move |_| spawn_local(async move {
                                        match pause_tournament(device_id, timer_id).await {
                                            Ok(_) => {}
                                            Err(e) => {
                                                settable_state.set(TimerCompState::Error(e.to_string()))
                                            }
                                        }
                                    })>"Pause"</button>
                                }
                                    .into_any(),
                            )
                        }
                    }
                } else {
                    None
                }
            }}
        </Suspense>
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
) -> Result<Option<DeviceState>, ServerFnError> {
    crate::backend::current_state(device_id, timer_id).await
}

#[server]
pub async fn create_tournament(
    device_id: Uuid,
    timer_id: Uuid,
) -> Result<DeviceState, ServerFnError> {
    crate::backend::create_tournament(device_id, timer_id).await
}

fn register_service_worker(device_id: Uuid, timer_id: Uuid) {
    use js_sys::eval;
    let result = eval(&format!(
        "if ('serviceWorker' in navigator) {{
            navigator.serviceWorker.getRegistration()
            .then(registration => {{
                console.log('Service Worker registered:', registration);
                return registration.pushManager.subscribe({{
                    userVisibleOnly: true,
                    applicationServerKey: 'BM7EadIlCgfqJABkpI9L0OsbkyZfL1BnEzjBlYpPAoZt-kDpByG3waoERsCLofkeqRsFBRfbgdJ7ccbSb_oxBf8'
                }});
             }}).then(subscription => {{
                fetch('/subscribe/{timer_id}', {{
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
                fetch('/unsubscribe/{timer_id}', {{
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

#[server]
async fn resume_tournament(device_id: Uuid, timer_id: Uuid) -> Result<(), ServerFnError> {
    crate::backend::resume_tournament(device_id, timer_id).await
}

#[server]
async fn pause_tournament(device_id: Uuid, timer_id: Uuid) -> Result<(), ServerFnError> {
    crate::backend::pause_tournament(device_id, timer_id).await
}
