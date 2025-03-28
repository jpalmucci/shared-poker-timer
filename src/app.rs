//! All the front end code

use crate::model::*;
use codee::string::JsonSerdeCodec;
use lazy_regex::regex;
use leptos::{logging::error, prelude::*, task::spawn_local};
// https://carloskiki.github.io/icondata/
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

        // content for this welcome page
        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=HomePage />
                    <Route path=path!("/:timer_id/timer/:timer_name") view=TimerPage />
                    <Route path=path!("/:timer_id/settings/:timer_name") view=SettingsPage />
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
    // TODO - make this an environment variable
    let re = regex!(r#"^https://pokertimer.palmucci.net/([^/]+)/timer/(.*)$"#);

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

#[derive(Params, PartialEq, Clone, Debug)]
struct TimerPageParams {
    timer_id: Option<Uuid>,
    timer_name: Option<String>,
}

fn extract_params() -> Result<(Uuid, String), String> {
    let params = use_params::<TimerPageParams>();
    match params.get() {
        Ok(TimerPageParams {
            timer_id: Some(timer_id),
            timer_name: Some(timer_name),
        }) => Ok((timer_id, timer_name)),
        _ => Err(format!("Bad Request {:?}", params.get())),
    }
}

#[component]
fn TimerPage() -> impl IntoView {
    view! {
        <CloseButton href=None />
        {|| {
            match extract_params() {
                Ok((timer_id, timer_name)) => {
                    view! {
                        <Link
                            rel="manifest"
                            href=format!("/{timer_id}/{timer_name}/manifest.json")
                        />
                        <Title text=format!("{timer_name} Poker Timer") />
                        <TimerComp timer_id=timer_id timer_name=timer_name />
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

#[cfg(not(feature = "ssr"))]
fn get_device_id() -> Uuid {
    fn set_id(storage: &web_sys::Storage, id: Uuid) -> Uuid {
        storage
            .set_item("deviceid", &id.to_string())
            .expect("Couldn't set device_id");
        id
    }
    if let Ok(Some(storage)) = window().local_storage() {
        match storage.get_item("deviceid") {
            Ok(Some(item)) => {
                match Uuid::parse_str(&item) {
                    Ok(id) => id,
                    Err(_) => {
                        // value was unparseable, set a new value
                        set_id(&storage, Uuid::new_v4())
                    }
                }
            }
            _ => {
                // id wasn't there, make a new one
                set_id(&storage, Uuid::new_v4())
            }
        }
    } else {
        // no local storage, punt
        Uuid::nil()
    }
}

#[cfg(feature = "ssr")]
fn get_device_id() -> Uuid {
    Uuid::nil()
}

#[component]
fn TimerComp(timer_id: Uuid, timer_name: String) -> impl IntoView {
    maybe_add_timer(timer_id, &timer_name);
    let encoded_name = urlencoding::encode(&timer_name).into_owned();
    let device_id = get_device_id();
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
    let structures = Resource::new(|| (), |_| structure_names());
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
                                            create_tournament(timer_id, selected_structure.get())
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
                                            {if let Some(Ok(x)) = structures.get() {
                                                x.iter()
                                                    .map(|name| {
                                                        view! { <option value=name.clone()>{name.clone()}</option> }
                                                    })
                                                    .collect_view()
                                            } else {
                                                Vec::new()
                                            }}
                                        </select>
                                    </div>
                                    <button type="submit">Start</button>
                                </form>
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
                            <SettingsButton timer_id=timer_id timer_name=timer_name.clone() />
                            <div class="title">
                                {
                                    let timer_name = timer_name.clone();
                                    move || { timer_name.clone() }
                                }
                            </div>

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
                                    prop:checked=subscribed
                                    on:input:target=move |evt| {
                                        evt.prevent_default();
                                        if evt.target().checked() {
                                            register_service_worker(device_id, timer_id);
                                        } else {
                                            deregister_service_worker(device_id, timer_id);
                                        }
                                    }
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
    device_id: Uuid,
    timer_id: Uuid,
) -> Result<TimerCompState, ServerFnError> {
    crate::timers::current_state(device_id, timer_id).await
}

#[server]
pub async fn create_tournament(
    timer_id: Uuid,
    structure_name: String,
) -> Result<(), ServerFnError> {
    crate::timers::create_tournament(timer_id, structure_name).await
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
    new Audio('/beep.mp3').play();
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
            if let Ok((timer_id, _)) = params {
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
                Ok((timer_id, timer_name)) => {
                    let execute_command = {
                        let timer_name = timer_name.clone();
                        move |cmd| {
                            let timer_name = timer_name.clone();
                            spawn_local(async move {
                                if let Ok(_) = execute_command(cmd, timer_id, device_id).await {
                                    use_navigate()(
                                        &format!("/{}/timer/{}", timer_id, timer_name),
                                        NavigateOptions::default(),
                                    );
                                }
                            });
                        }
                    };
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
    crate::timers::set_tournament_settings(timer_id, duration_override)
}

#[server]
async fn tournament_settings(timer_id: Uuid) -> Result<Option<Duration>, ServerFnError> {
    return crate::timers::tourament_settings(timer_id);
}

#[server]
async fn execute_command(
    cmd: Command,
    timer_id: Uuid,
    device_id: Uuid,
) -> Result<(), ServerFnError> {
    crate::timers::execute(&cmd, timer_id, device_id);
    Ok(())
}

#[server]
async fn structure_names() -> Result<Vec<String>, ServerFnError> {
    Ok(crate::structures::STRUCTURES
        .keys()
        .map(|x| x.clone())
        .collect())
}
