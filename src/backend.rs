use crate::model::*;
use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use axum::extract::Path;
use axum::extract::WebSocketUpgrade;
use axum::http::header;
use axum::response::IntoResponse;
use chrono::Duration;
use codee::binary::MsgpackSerdeCodec;
use codee::Encoder;
use dashmap::DashMap;
use image::Luma;
use leptos::prelude::ServerFnError;
use leptos::prelude::StorageAccess;
use log::{error, info};
use once_cell::sync::Lazy;
use qrcode::QrCode;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::RwLockWriteGuard;
use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::sync::Arc;
use tokio::sync::RwLockReadGuard;
use tokio::sync::RwLock;
use tokio::time::sleep;
use web_push::{
    ContentEncoding, IsahcWebPushClient, SubscriptionInfo, VapidSignatureBuilder, WebPushClient,
    WebPushMessageBuilder,
};

use uuid::Uuid;

pub static STRUCTURE: Lazy<Arc<Structure>> = Lazy::new(|| Arc::new(Structure::new()));

pub static TIMERS: Lazy<(Uuid,RwLock<HashMap<Uuid, Tournament>>)> = Lazy::new(|| {
    (Uuid::new_v4(),RwLock::new(HashMap::new()))
});

async fn read_timers() -> RwLockReadGuard<'static, HashMap<Uuid, Tournament>>
{
    info!("timer id {}", TIMERS.0);
    TIMERS.1.read().await
}

async fn write_timers() -> RwLockWriteGuard<'static, HashMap<Uuid, Tournament>>
{
    TIMERS.1.write().await
}

fn read_timers_blocking() -> RwLockReadGuard<'static, HashMap<Uuid, Tournament>>
{
    TIMERS.1.blocking_read()
}

fn write_timers_blocking() -> RwLockWriteGuard<'static, HashMap<Uuid, Tournament>>
{
    TIMERS.1.blocking_write()
}

pub struct Tournament {
    timer_id: Uuid,
    structure: Arc<Structure>,
    level: usize,
    clock_state: ClockState,
    settings: TournamentSettings,
    subscriptions: HashMap<Uuid, Subscription>,
    // contains the message and the device ID responsible for the message (if there is one)
    event_sender: async_broadcast::Sender<(TournamentMessage, Option<Uuid>)>,
}

impl Drop for Tournament {
    fn drop(&mut self) {
        info!("Dropping {}", self.timer_id);
        todo!()
    }
}

impl Tournament {
    pub fn new(timer_id: Uuid) -> Tournament {
        let str = STRUCTURE.clone();
        let (tx, mut rx) = async_broadcast::broadcast(100);
        let mut rx2 = tx.new_receiver();
        let tournament = Tournament {
            timer_id: timer_id,
            structure: STRUCTURE.clone(),
            level: 1,
            clock_state: ClockState::Paused {
                remaining: str.get_level(1).duration(),
            },
            subscriptions: HashMap::new(),
            settings: TournamentSettings::default(),
            event_sender: tx,
        };

        tokio::spawn(async move {
            loop {
                info!(
                    "bg: Timer {} exists {}",
                    timer_id,
                    read_timers().await.contains_key(&timer_id)
                );
                sleep(std::time::Duration::from_secs(2)).await;
            }
        });
        // start a thread to do the level changes
        // TODO - give a one minute warning before end of break
        tokio::spawn(async move {
            loop {
                {
                    let time = match read_timers().await.get(&timer_id) {
                        None => {
                            // the tournament ended
                            break;
                        }
                        Some(tournament) => std::time::Duration::from_millis(
                            tournament.clock_state.remaining().num_milliseconds() as u64,
                        ),
                    };
                    // wait until the time has elapsed, or a message that changed the state of the
                    // tournament occurred.
                    tokio::select! {
                        _ = sleep(time) => {},
                        _ = rx.recv() => {}
                    }
                }

                {
                    match write_timers().await.get_mut(&timer_id) {
                        None => {
                            // the tournament ended
                            break;
                        }
                        Some(mut tournament) => {
                            if tournament.clock_state.remaining().num_seconds() < 2 {
                                if *tournament.cur_level() == Level::Done {
                                    // we are done and the grace period has expired. Delete the tournament
                                    break;
                                }
                                tournament.level_up();
                            }
                        }
                    }
                }
            }
            info!("Deleting tournament {timer_id}");
            write_timers().await.remove(&timer_id);
        });

        // start a thread to do the broadcasting
        tokio::spawn(async move {
            loop {
                let message = rx2.recv().await;
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
                        });
                        let subscriptions = match read_timers().await.get(&timer_id) {
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
        return tournament;
    }

    fn cur_level<'a>(&'a self) -> &'a Level {
        &self.structure.get_level(self.level)
    }

    fn level_up(&mut self) {
        let cur = self.cur_level();
        if *cur == Level::Done {
            // nothing to do here. Already done
            return;
        }
        self.level += 1;
        let cur = &self.structure.get_level(self.level);
        if let Level::Done = cur {
            // if we are done, have the url hang around for 15 minutes and then die
            self.clock_state = ClockState::Running {
                // TODO fixme
                remaining: Duration::seconds(1),
                asof: chrono::Local::now(),
            };
        } else {
            let duration = match self.settings.duration_override {
                Some(duration) => duration,
                None => cur.duration(),
            };
            self.clock_state = ClockState::Running {
                remaining: duration,
                asof: chrono::Local::now(),
            };
        }
        self.broadcast(None, TournamentMessage::LevelUp(self.to_roundstate()));
    }

    fn update_settings(&mut self, settings: TournamentSettings) {
        // if the round duration is changing, update the clock_state
        let cur = &self.structure.get_level(self.level);
        let current_duration = match self.settings.duration_override {
            Some(d) => d,
            None => cur.duration(),
        };
        let new_duration = match settings.duration_override {
            Some(d) => d,
            None => cur.duration(),
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
        self.settings = settings;
        self.broadcast(None, TournamentMessage::Settings(self.to_roundstate()));
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

    fn to_roundstate(&self) -> RoundState {
        RoundState {
            timer_id: self.timer_id,
            level: self.level,
            cur: self.structure.get_level(self.level).clone(),
            next: self.structure.get_level(self.level + 1).clone(),
            clock: self.clock_state.clone(),
        }
    }

    fn to_devicestate(&self, device: Uuid) -> DeviceState {
        let subscribed = self.subscriptions.contains_key(&device);
        DeviceState {
            subscribed,
            state: self.to_roundstate(),
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
            return &Level::Done;
        }
        return &self.levels[l - 1];
    }
    pub fn new() -> Self {
        Self {
            levels: vec![
                Level::Blinds {
                    small: 100,
                    big: 200,
                    ante: 200,
                    duration: chrono::Duration::seconds(20),
                },
                Level::Blinds {
                    small: 200,
                    big: 300,
                    ante: 300,
                    duration: chrono::Duration::seconds(20),
                },
                Level::Done,
                Level::Blinds {
                    small: 200,
                    big: 400,
                    ante: 400,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    small: 300,
                    big: 500,
                    ante: 500,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    small: 300,
                    big: 600,
                    ante: 600,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Break {
                    duration: chrono::Duration::minutes(10),
                },
                Level::Blinds {
                    small: 400,
                    big: 800,
                    ante: 800,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    small: 500,
                    big: 1000,
                    ante: 1000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    small: 600,
                    big: 1200,
                    ante: 1200,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    small: 1000,
                    big: 1500,
                    ante: 1500,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    small: 1000,
                    big: 2000,
                    ante: 2000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    small: 1500,
                    big: 2500,
                    ante: 2500,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    small: 1500,
                    big: 3000,
                    ante: 3000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    small: 2000,
                    big: 4000,
                    ante: 4000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    small: 2500,
                    big: 5000,
                    ante: 5000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    small: 3000,
                    big: 6000,
                    ante: 6000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    small: 4000,
                    big: 8000,
                    ante: 8000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    small: 5000,
                    big: 10000,
                    ante: 10000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    small: 6000,
                    big: 12000,
                    ante: 12000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    small: 10000,
                    big: 15000,
                    ante: 15000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Blinds {
                    small: 10000,
                    big: 20000,
                    ante: 20000,
                    duration: chrono::Duration::minutes(20),
                },
                Level::Done,
            ],
        }
    }
}

pub async fn create_tournament(
    device_id: Uuid,
    timer_id: Uuid,
) -> Result<DeviceState, ServerFnError> {
    if let Some(tournament) = read_timers().await.get(&timer_id) {
        return Ok(tournament.to_devicestate(device_id));
    }
    let device_state = {
        info!("Creating timer {timer_id}");
        let tournament = Tournament::new(timer_id);
        let device_state = tournament.to_devicestate(device_id);
        write_timers().await.insert(timer_id, tournament);
        device_state
    };
    let n = read_timers().await.len();
    info!("Timer created, there are {n} timers.");
    Ok(device_state)
}

pub async fn current_state(
    device_id: Uuid,
    timer_id: Uuid,
) -> Result<Option<DeviceState>, ServerFnError> {
    info!("cs: There are {} timers", read_timers().await.len());
    match read_timers().await.get(&timer_id) {
        None => Ok(None),
        Some(tournament) => Ok(Some(tournament.to_devicestate(device_id))),
    }
}

pub async fn resume_tournament(device_id: Uuid, timer_id: Uuid) -> Result<(), ServerFnError> {
    info!("rt: There are {} timers", read_timers().await.len());
    match write_timers().await.get_mut(&timer_id) {
        None => {
            error!("resumed, timer not found");
            Err(ServerFnError::Args("not found".to_string()))
        }
        Some(mut tournament) => {
            tournament.clock_state = tournament.clock_state.resume();
            tournament.broadcast(
                Some(device_id),
                TournamentMessage::Resume(tournament.to_roundstate()),
            );
            Ok(())
        }
    }
}

pub async fn pause_tournament(device_id: Uuid, timer_id: Uuid) -> Result<(), ServerFnError> {
    match write_timers().await.get_mut(&timer_id) {
        None => {
            error!("Paused, timer not found");
            Err(ServerFnError::Args("not found".to_string()))
        }
        Some(tournament) => {
            tournament.clock_state = tournament.clock_state.pause();
            let state = tournament.to_roundstate();
            tournament.broadcast(Some(device_id), TournamentMessage::Pause(state.clone()));
            Ok(())
        }
    }
}

pub fn tourament_settings(timer_id: Uuid) -> Result<Option<TournamentSettings>, ServerFnError> {
    match read_timers_blocking().get(&timer_id) {
        None => {
            return Ok(None);
        }
        Some(t) => {
            return Ok(Some(t.settings));
        }
    }
}

pub fn set_tournament_settings(
    timer_id: Uuid,
    settings: TournamentSettings,
) -> Result<(), ServerFnError> {
    match write_timers_blocking().get_mut(&timer_id) {
        None => {
            return Err(ServerFnError::new("Tournament not running"));
        }
        Some(t) => {
            t.update_settings(settings);
            return Ok(());
        }
    }
}

async fn handle_socket(timer_id: Uuid, mut socket: WebSocket) {
    let x = read_timers().await;
    info!(
        "hs: Timer {} exists {}",
        timer_id,
        read_timers().await.contains_key(&timer_id)
    );

    // let channel = match TIMERS.get(&timer_id) {
    //     Some(timer) => timer.event_sender.new_receiver(),
    //     None => {
    //         info!("No timer {timer_id}");
    //         return;
    //     }
    // };
    // loop {
    //     tokio::select! {
    //      x = channel.recv() => {
    //         match x {
    //             Ok((tm, _sender)) => {
    //                 info!("Sending Message");
    //                 let dm = DeviceMessage{ msg: tm, subscribed: false };
    //                 let message = MsgpackSerdeCodec::encode(&dm).expect("Couldn't encode");
    //                 if let Err(e) = socket.send(Message::Binary(message)).await {
    //                     info!("couldn't send {e}");
    //                     return;
    //                 }
    //             },
    //             Err(e) => {
    //                 info!("Error reading channel {e}");
    //                 return;
    //             }
    //         }
    //     },

    //     x = socket.recv() => {
    //         match x {
    //             None => { return; },
    //             Some(msg) => {
    //                 info!("{msg:?}");
    //             }
    //         }
    //     }
    //     }
    // }
}

pub async fn websocket_handler(
    Path(timer_id): Path<Uuid>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(async move |socket| handle_socket(timer_id, socket).await)
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
    let url = format!("https://pokertimer.palmucci.net/timer/{timer_id}/{timer_name}");
    let code = QrCode::new(url).unwrap();
    let image = code.render::<Luma<u8>>().module_dimensions(4, 4).build();
    let mut buf = Cursor::new(Vec::new());
    image.write_to(&mut buf, image::ImageFormat::Png).unwrap();

    // Return as a PNG image response
    ([(header::CONTENT_TYPE, "image/png")], buf.into_inner())
}
