use crate::app::shell;
use crate::app::App;
use crate::model::*;
use axum::extract;
use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use axum::extract::Path;
use axum::extract::WebSocketUpgrade;
use axum::http::header;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::Json;
use chrono::Duration;
use codee::string::JsonSerdeWasmCodec;
use codee::Encoder;
use dashmap::DashMap;
use image::Luma;
use leptos::prelude::ServerFnError;
use log::{error, info};
use once_cell::sync::Lazy;
use qrcode::QrCode;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::sync::Arc;
use tokio::time::sleep;
use uuid::Uuid;
use web_push::{
    ContentEncoding, IsahcWebPushClient, SubscriptionInfo, VapidSignatureBuilder, WebPushClient,
    WebPushMessageBuilder,
};

pub async fn main() {
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
        .route("/:timer_id/qr/:timer_name", get(qr_code))
        .route("/:timer_id/ws/:device_id", any(websocket_handler))
        .route("/:timer_id/subscribe", post(subscribe))
        .route("/:timer_id/unsubscribe", post(unsubscribe))
        .route("/:timer_id/:timer_name/manifest.json", get(manifest))
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

pub static STRUCTURE: Lazy<HashMap<String, Arc<Structure>>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert(
        "XNPG Nightly TOC".to_string(),
        Arc::new(Structure {
            levels: vec![
                Level::Limit {
                    game: "Hold Em".to_string(),
                    small: 200,
                    big: 400,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Limit {
                    game: "Omaha Hi/Lo".to_string(),
                    small: 200,
                    big: 500,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Stud {
                    game: "Stud Hi/Lo".to_string(),
                    ante: 100,
                    bring_in: 200,
                    small: 600,
                    big: 1200,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Limit {
                    game: "Hold Em".to_string(),
                    small: 400,
                    big: 800,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Limit {
                    game: "Omaha Hi/Lo".to_string(),
                    small: 500,
                    big: 1000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Stud {
                    game: "Stud Hi/Lo".to_string(),
                    ante: 300,
                    bring_in: 400,
                    small: 1200,
                    big: 2400,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Break {
                    duration: chrono::Duration::minutes(10),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 500,
                    big: 1000,
                    ante: 1000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 600,
                    big: 1200,
                    ante: 1200,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 1000,
                    big: 1500,
                    ante: 1500,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 1000,
                    big: 2000,
                    ante: 2000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 1500,
                    big: 2500,
                    ante: 2500,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 1500,
                    big: 3000,
                    ante: 3000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 2000,
                    big: 4000,
                    ante: 4000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 2500,
                    big: 5000,
                    ante: 5000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 3000,
                    big: 6000,
                    ante: 6000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 4000,
                    big: 8000,
                    ante: 8000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 5000,
                    big: 10000,
                    ante: 10000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 6000,
                    big: 12000,
                    ante: 12000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 8000,
                    big: 16000,
                    ante: 16000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 10000,
                    big: 20000,
                    ante: 20000,
                    duration: chrono::Duration::minutes(20),
                },
            ],
        }),
    );

    map.insert(
        "XNPG Nightly NLHE".to_string(),
        Arc::new(Structure {
            levels: vec![
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 100,
                    big: 200,
                    ante: 200,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 200,
                    big: 300,
                    ante: 300,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 200,
                    big: 400,
                    ante: 400,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 300,
                    big: 500,
                    ante: 500,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 300,
                    big: 600,
                    ante: 600,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Break {
                    duration: chrono::Duration::minutes(10),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 400,
                    big: 800,
                    ante: 800,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 500,
                    big: 1000,
                    ante: 1000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 600,
                    big: 1200,
                    ante: 1200,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 1000,
                    big: 1500,
                    ante: 1500,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 1000,
                    big: 2000,
                    ante: 2000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 1500,
                    big: 2500,
                    ante: 2500,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 1500,
                    big: 3000,
                    ante: 3000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 2000,
                    big: 4000,
                    ante: 4000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 2500,
                    big: 5000,
                    ante: 5000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 3000,
                    big: 6000,
                    ante: 6000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 4000,
                    big: 8000,
                    ante: 8000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 5000,
                    big: 10000,
                    ante: 10000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 6000,
                    big: 12000,
                    ante: 12000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 10000,
                    big: 15000,
                    ante: 15000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 10000,
                    big: 20000,
                    ante: 20000,
                    duration: chrono::Duration::minutes(20),
                },
            ],
        }),
    );

    map
});

pub static TIMERS: Lazy<DashMap<Uuid, Timer>> = Lazy::new(|| DashMap::new());

pub struct Timer {
    timer_id: Uuid,
    subscriptions: HashMap<Uuid, Subscription>,
    // contains the message and the device ID responsible for the message (if there is one)
    event_sender: async_broadcast::Sender<(TournamentMessage, Option<Uuid>)>,
    tournament: Option<Tournament>,
}

impl Timer {
    pub fn new(timer_id: Uuid) -> Timer {
        let (tx, mut rx) = async_broadcast::broadcast(100);
        let new_timer = Timer {
            timer_id: timer_id.clone(),
            subscriptions: HashMap::new(),
            event_sender: tx,
            tournament: None,
        };

        // start a thread to do the broadcasting
        tokio::spawn(async move {
            loop {
                let message = rx.recv().await;
                match message {
                    Err(e) => match e {
                        async_broadcast::RecvError::Overflowed(_) => {
                            info!("Unexpected error on channel: {e}")
                        }
                        async_broadcast::RecvError::Closed => {
                            // the channel is closed, this tournament has ended
                            break;
                        }
                    },

                    Ok((message, from_device_id)) => {
                        let notification = Arc::new(match &message {
                            TournamentMessage::Hello(_) => Notification {
                                title: "Hello".to_string(),
                                body: "Notifications are on".to_string(),
                            },

                            TournamentMessage::Pause(_) => Notification {
                                title: "Update".to_string(),
                                body: "Tournament Paused".to_string(),
                            },
                            TournamentMessage::Resume(_) => Notification {
                                title: "Update".to_string(),
                                body: "Tournament Resumed".to_string(),
                            },
                            TournamentMessage::LevelUp(round_state) => {
                                let level = round_state.cur.make_display_string();
                                Notification {
                                    title: "Update".to_string(),
                                    body: format!["Level Up: {level}"],
                                }
                            }
                            TournamentMessage::Settings(_) => Notification {
                                title: "Update".to_string(),
                                body: "Tournament settings have changed".to_string(),
                            },
                            // this doesnt result in a notification
                            TournamentMessage::Goodbye => Notification {
                                title: "Update".to_string(),
                                body: "Tournament has been terminated".to_string(),
                            },
                            // this doesnt result in a notification
                            TournamentMessage::SubscriptionChange(_) => continue,
                        });
                        let subscriptions = match TIMERS.get(&timer_id) {
                            Some(tournament) => tournament.subscriptions.clone(),
                            None => break,
                        };

                        futures::future::join_all(
                            subscriptions
                                .iter()
                                .filter(|(device_id, _sub)| match from_device_id {
                                    Some(from_device_id) => **device_id != from_device_id,
                                    None => true,
                                })
                                .map(|(_device_id, sub)| {
                                    send_notification(sub, notification.clone())
                                }),
                        )
                        .await;
                    }
                }
            }
        });
        new_timer
    }
    fn make_tournament(&mut self, structure: &Arc<Structure>) {
        if self.tournament.is_none() {
            let tournament = Tournament::new(self, structure);
            let message = TournamentMessage::Hello(tournament.to_roundstate());
            self.tournament = Some(tournament);
            (&*self).broadcast(None, message);
        }
    }

    fn broadcast(&self, from_device_id: Option<Uuid>, message: TournamentMessage) {
        let result = self
            .event_sender
            .try_broadcast((message.clone(), from_device_id));
        match result {
            Ok(_) => {}
            Err(err) => {
                error!["error broadcasting message: {err:?}",]
            }
        }
    }
    /// return true if the level is done
    fn level_up(&mut self, delta: i8) -> bool {
        let result = match self.tournament {
            None => {
                return true;
            }
            Some(ref mut tournament) => tournament.level_up(delta),
        };
        match result {
            LevelUpResult::Invalid => false,
            LevelUpResult::Done => {
                self.tournament = None;
                (&*self).broadcast(None, TournamentMessage::Goodbye);
                true
            }
            LevelUpResult::Ok => {
                let message =
                    TournamentMessage::LevelUp(self.tournament.as_ref().unwrap().to_roundstate());
                (&*self).broadcast(None, message);
                false
            }
        }
    }

    fn terminate(&mut self) {
        self.tournament = None;
        (&*self).broadcast(None, TournamentMessage::Goodbye);
    }

    fn to_timer_comp_state(&self, device: &Uuid) -> TimerCompState {
        if let Some(tournament) = &self.tournament {
            let subscribed = self.subscriptions.contains_key(&device);
            TimerCompState::Running {
                subscribed,
                state: tournament.to_roundstate(),
            }
        } else {
            TimerCompState::NoTournament
        }
    }
    fn update_settings(&mut self, duration_override: Option<Duration>) {
        if let Some(ref mut tournament) = &mut self.tournament {
            tournament.update_settings(duration_override);
            let message = TournamentMessage::Settings(tournament.to_roundstate());
            (&*self).broadcast(None, message);
        }
    }

    fn resume_tournament(&mut self, device_id: Uuid) {
        if let Some(ref mut tournament) = &mut self.tournament {
            tournament.clock_state = tournament.clock_state.resume();
            let message = TournamentMessage::Resume(tournament.to_roundstate());
            (&*self).broadcast(Some(device_id), message);
        }
    }

    fn pause_tournament(&mut self, device_id: Uuid) {
        if let Some(ref mut tournament) = &mut self.tournament {
            tournament.clock_state = tournament.clock_state.pause();
            let message = TournamentMessage::Pause(tournament.to_roundstate());
            (&*self).broadcast(Some(device_id), message);
        }
    }
}

pub struct Tournament {
    timer_id: Uuid,
    structure: Arc<Structure>,
    level: usize,
    clock_state: ClockState,
    duration_override: Option<Duration>,
}
// return true if the tournament is complete
enum LevelUpResult {
    Done,
    Invalid,
    Ok,
}

impl Tournament {
    pub fn new(timer: &Timer, structure: &Arc<Structure>) -> Tournament {
        let mut rx = timer.event_sender.new_receiver();
        let timer_id = timer.timer_id;
        let tournament = Tournament {
            timer_id: timer.timer_id,
            structure: structure.clone(),
            level: 1,
            clock_state: ClockState::Paused {
                remaining: structure.get_level(1).duration(),
            },
            duration_override: None,
        };

        // start a thread to do the level changes
        // TODO - give a one minute warning before end of break
        tokio::spawn(async move {
            loop {
                {
                    let time = match TIMERS.get(&timer_id) {
                        None => {
                            // the tournament ended
                            break;
                        }
                        Some(timer) => match &*timer {
                            Timer {
                                tournament: Some(tournament),
                                ..
                            } => std::time::Duration::from_millis(
                                tournament.clock_state.remaining().num_milliseconds() as u64,
                            ),
                            _ => {
                                // the tournament ended
                                break;
                            }
                        },
                    };
                    // wait until the time has elapsed, or a message that changed the state of the
                    // tournament occurred.
                    tokio::select! {
                        _ = sleep(time) => {},
                        _ = rx.recv() => {}
                    }
                }

                {
                    match TIMERS.get_mut(&timer_id) {
                        None => {
                            // the tournament ended
                            break;
                        }
                        Some(mut timer) => {
                            match &mut *timer {
                                Timer {
                                    tournament: Some(tournament),
                                    ..
                                } => {
                                    if tournament.clock_state.remaining().num_seconds() < 2 {
                                        let done = timer.level_up(1);
                                        if done {
                                            break;
                                        }
                                    }
                                }
                                _ => {
                                    // the tournament ended
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            info!("Deleting tournament {timer_id}");
            // FIXME - where should we delete these (or just let them sit?)
            // TIMERS.remove(&timer_id);
        });

        return tournament;
    }

    fn level_up(&mut self, delta: i8) -> LevelUpResult {
        if self.level as i8 + delta < 0 {
            return LevelUpResult::Invalid;
        }
        self.level += 1;
        let level = self.structure.get_level(self.level);
        if level == &Level::Done {
            return LevelUpResult::Done;
        }
        let duration = match self.duration_override {
            Some(duration) => duration,
            None => level.duration(),
        };
        self.clock_state = ClockState::Running {
            remaining: duration,
            asof: chrono::Local::now(),
        };
        LevelUpResult::Ok
    }

    fn update_settings(&mut self, duration_override: Option<Duration>) {
        // if the round duration is changing, update the clock_state
        let level = self.structure.get_level(self.level);
        let current_duration = match self.duration_override {
            Some(d) => d,
            None => level.duration(),
        };
        let new_duration = match duration_override {
            Some(d) => d,
            None => level.duration(),
        };
        let offset = new_duration - current_duration;
        match self.clock_state {
            ClockState::Paused { remaining } => {
                self.clock_state = ClockState::Paused {
                    remaining: remaining + offset,
                };
            }
            ClockState::Running { remaining, asof } => {
                self.clock_state = ClockState::Running {
                    remaining: remaining + offset,
                    asof,
                };
            }
        }
        self.duration_override = duration_override;
    }

    fn to_roundstate(&self) -> RoundState {
        RoundState {
            timer_id: self.timer_id,
            level: self.level,
            cur: self.structure.get_level(self.level).clone(),
            next: self.structure.get_level(self.level + 1).clone(),
            clock: self.clock_state.clone(),
        }
    }
}

// These implementations are in the backend explicitly because the front
// end should never be doing this
impl ClockState {
    pub(self) fn pause(&self) -> ClockState {
        match self {
            Self::Paused { .. } => *self,
            Self::Running { remaining, asof } => Self::Paused {
                remaining: *remaining - chrono::Local::now().signed_duration_since(asof),
            },
        }
    }
    pub(self) fn resume(&self) -> ClockState {
        match self {
            Self::Paused { remaining } => Self::Running {
                remaining: *remaining,
                asof: chrono::Local::now(),
            },
            Self::Running { .. } => *self,
        }
    }
}

#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Structure {
    pub levels: Vec<Level>,
}

impl Structure {
    pub fn get_level<'a>(&'a self, l: usize) -> &'a Level {
        if l >= self.levels.len() {
            &Level::Done
        } else {
            &self.levels[l - 1]
        }
    }
}

pub async fn create_tournament(timer_id: Uuid) -> Result<(), ServerFnError> {
    // make the timer if it does not exist yet
    if let Some(timer) = TIMERS.get(&timer_id) {
        if timer.tournament.is_some() {
            return Ok(());
        }
    } else {
        // FIXME - race condition?
        TIMERS.insert(timer_id, Timer::new(timer_id));
    }

    // if we are here, we have a timer with tournament = None
    if let Some(mut timer) = TIMERS.get_mut(&timer_id) {
        info!("Creating tournament {timer_id}");
        let structure = STRUCTURE.get("XNPG Nightly TOC");
        match structure {
            Some(structure) => {
                timer.make_tournament(structure);
                return Ok(());
            }
            None => {
                return Err(ServerFnError::new("Structure not found"));
            }
        }
    } else {
        return Err(ServerFnError::new(
            "Someone deleted the timer as we were creating the tournament",
        ));
    }
}

pub async fn current_state(
    device_id: Uuid,
    timer_id: Uuid,
) -> Result<TimerCompState, ServerFnError> {
    match TIMERS.get(&timer_id) {
        None => {
            TIMERS.insert(timer_id, Timer::new(timer_id));
            Ok(TimerCompState::NoTournament)
        }
        Some(tournament) => Ok(tournament.to_timer_comp_state(&device_id)),
    }
}

pub fn tourament_settings(timer_id: Uuid) -> Result<Option<Duration>, ServerFnError> {
    match TIMERS.get(&timer_id) {
        None => {
            return Err(ServerFnError::new("running tournament"));
        }
        Some(timer) => match &timer.tournament {
            Some(t) => Ok(t.duration_override),
            None => Err(ServerFnError::new("running tournament")),
        },
    }
}

pub fn set_tournament_settings(
    timer_id: Uuid,
    duration_override: Option<Duration>,
) -> Result<(), ServerFnError> {
    match TIMERS.get_mut(&timer_id) {
        None => {
            return Err(ServerFnError::new("Tournament not running"));
        }
        Some(mut t) => {
            t.update_settings(duration_override);
            return Ok(());
        }
    }
}

async fn handle_socket(timer_id: Uuid, device_id: Uuid, mut socket: WebSocket) {
    let (mut channel, hello) = match TIMERS.get(&timer_id) {
        Some(timer) => (timer.event_sender.new_receiver(), {
            DeviceMessage::NewState(timer.to_timer_comp_state(&device_id))
        }),
        None => {
            let timer = Timer::new(timer_id);
            let channel = timer.event_sender.new_receiver();
            let hello = DeviceMessage::NewState(timer.to_timer_comp_state(&device_id));
            TIMERS.insert(timer_id, timer);
            (channel, hello)
        }
    };
    let message = JsonSerdeWasmCodec::encode(&hello).expect("Couldn't encode");
    match socket.send(Message::Text(message)).await {
        Ok(_) => {}
        Err(e) => {
            info!("couldn't send hello {e}");
            return;
        }
    }
    loop {
        tokio::select! {
         x = channel.recv() => {
            match x {
                Ok((tm, _sender)) => {
                    if let TournamentMessage::SubscriptionChange(changing_device) = tm {
                        // this only changes the state of the device that is changing its subscription
                        if changing_device != device_id {
                            continue;
                        }
                    }
                    if let TournamentMessage::LevelUp(_) = tm {
                        let message = JsonSerdeWasmCodec::encode(&DeviceMessage::Beep).expect("Couldn't encode");
                        if let Err(e) = socket.send(Message::Text(message)).await {
                            info!("couldn't send {e}");
                            break;
                        }
                    }
                    let message = match TIMERS.get(&timer_id) {
                        Some(timer) => timer.to_timer_comp_state(&device_id),
                        None => TimerCompState::NoTournament
                    };
                    let message = JsonSerdeWasmCodec::encode(&DeviceMessage::NewState(message)).expect("Couldn't encode");
                    if let Err(e) = socket.send(Message::Text(message)).await {
                        info!("couldn't send {e}");
                        break;
                    }
                },
                Err(e) => {
                    info!("Error reading channel {e}");
                    break;
                }
            }
        },

        x = socket.recv() => {
            match x {
                Some(Ok(Message::Text(msg))) => {
                    match serde_json::from_str::<Command>(&msg) {
                        Ok(Command::Resume) =>  {
                            if let Some(mut timer) = TIMERS.get_mut(&timer_id) {
                                timer.resume_tournament(device_id);
                            }
                        },
                        Ok(Command::Pause) =>  {
                            if let Some(mut timer) = TIMERS.get_mut(&timer_id) {
                                timer.pause_tournament(device_id);
                            }
                        },
                        Ok(Command::PrevLevel) =>  {
                            if let Some(mut timer) = TIMERS.get_mut(&timer_id) {
                                timer.level_up(-1);
                            }
                        },
                        Ok(Command::NextLevel) =>  {
                            if let Some(mut timer) = TIMERS.get_mut(&timer_id) {
                                timer.level_up(1);
                            }
                        },
                        Ok(Command::Terminate) =>  {
                            if let Some(mut timer) = TIMERS.get_mut(&timer_id) {
                                timer.terminate();
                            }
                        },

                        Err(_) => break
                    }
                },
                _ => { break; },
            }
        }
        }
    }
}

pub async fn websocket_handler(
    Path((timer_id, device_id)): Path<(Uuid, Uuid)>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(async move |socket| handle_socket(timer_id, device_id, socket).await)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Subscription {
    pub device_id: Uuid,
    pub endpoint: String,
    pub keys: SubscriptionKeys,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubscriptionKeys {
    pub auth: String,
    pub p256dh: String,
}

static WEB_SEND_CLIENT: Lazy<IsahcWebPushClient> = Lazy::new(|| IsahcWebPushClient::new().unwrap());

pub static NOTIFY_KEY: Lazy<String> = Lazy::new(|| {
    fs::read_to_string("certs/backend_notification_key.pem")
        .expect("Couldn't read backend_notification_key.pem")
});

#[derive(Serialize)]
pub struct Notification {
    pub title: String,
    pub body: String,
}

// TODO - make these be clickable to get back to the timer
pub async fn send_notification(s: &Subscription, notification: Arc<Notification>) -> () {
    let message = serde_json::to_string(&notification).unwrap();

    info!("Sending message {:?} {:?}", s, message);
    let subscription_info = SubscriptionInfo::new(
        s.endpoint.clone(),
        s.keys.p256dh.clone(),
        s.keys.auth.clone(),
    );

    let sig_builder =
        VapidSignatureBuilder::from_pem(NOTIFY_KEY.as_bytes(), &subscription_info).unwrap();

    let mut builder = WebPushMessageBuilder::new(&subscription_info);

    builder.set_payload(ContentEncoding::Aes128Gcm, message.as_bytes());
    builder.set_vapid_signature(sig_builder.build().unwrap());
    let message = builder.build().unwrap();

    match WEB_SEND_CLIENT.send(message).await {
        Ok(x) => {
            info!("Message send success: {:?}", x);
        }
        Err(e) => {
            info!("{:?}", e);
        }
    }
}

pub async fn qr_code(Path((timer_id, timer_name)): Path<(Uuid, String)>) -> impl IntoResponse {
    let timer_name = urlencoding::encode(&timer_name);
    let url = format!("https://pokertimer.palmucci.net/{timer_id}/timer/{timer_name}");
    let code = QrCode::new(url).unwrap();
    let image = code.render::<Luma<u8>>().module_dimensions(4, 4).build();
    let mut buf = Cursor::new(Vec::new());
    image.write_to(&mut buf, image::ImageFormat::Png).unwrap();

    // Return as a PNG image response
    ([(header::CONTENT_TYPE, "image/png")], buf.into_inner())
}

pub async fn subscribe(
    Path(timer_id): Path<Uuid>,
    extract::Json(payload): extract::Json<Subscription>,
) -> impl IntoResponse {
    match TIMERS.get_mut(&timer_id) {
        None => {
            return (StatusCode::NOT_FOUND, "Not found".to_string());
        }
        Some(mut t) => {
            let device_id = payload.device_id;
            t.subscriptions.remove(&device_id);
            t.subscriptions.insert(device_id, payload);
            info!("There are {} subscriptions", t.subscriptions.len());
            (&*t).broadcast(None, TournamentMessage::SubscriptionChange(device_id));
            return (StatusCode::OK, "ok".to_string());
        }
    }
}

pub async fn unsubscribe(
    Path(timer_id): Path<Uuid>,
    extract::Json(payload): extract::Json<Subscription>,
) -> Result<String, StatusCode> {
    let device_id = payload.device_id;
    match TIMERS.get_mut(&timer_id) {
        None => {
            return Err(StatusCode::NOT_FOUND);
        }
        Some(mut t) => {
            t.subscriptions.remove(&device_id);
            info!("There are {} subscriptions", t.subscriptions.len());
            (&*t).broadcast(None, TournamentMessage::SubscriptionChange(device_id));
            Ok("ok".to_string())
        }
    }
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
