//! The module is the main logic behind a virtual poker timer

use std::collections::HashMap;
use std::sync::Arc;

use crate::model::*;
use axum::extract::ws::{Message, WebSocket};
use codee::string::JsonSerdeWasmCodec;
use codee::Encoder;
use dashmap::DashMap;
use leptos::prelude::*;
use leptos::server_fn::error::ServerFnErrorErr;
use log::{error, info};
use once_cell::sync::Lazy;
use tokio::time::sleep;
use uuid::Uuid;

use crate::backend::{send_notification, Notification, Subscription};
use crate::persistence::StoredTournament;
use crate::structures::{Structure, STRUCTURES};

static TIMERS: Lazy<DashMap<Uuid, Timer>> = Lazy::new(|| DashMap::new());

// an internal message that is passed on the backend message bus
#[derive(Clone, serde::Deserialize, serde::Serialize, Debug)]
pub enum TournamentMessage {
    /// The given device just change notifications
    NotificationChange(Uuid),
    /// a new tournament started
    Started,
    /// the tournament ended
    Ended,
    Pause,
    Resume,
    LevelUp(RoundState),
    /// The tournament settings changed
    Settings,
    /// One minute warning till end of break
    OneMinuteWarning,
}

pub struct Timer {
    pub timer_id: Uuid,
    /// contains the message and the device ID responsible for the message (if there is one)
    /// this is useful if you want to stifle a PWA notification resulting from an action
    /// that user initialted (which would be annoying)
    pub event_sender: async_broadcast::Sender<(TournamentMessage, Option<Uuid>)>,
    /// The currently running tournament (if there is one)
    pub tournament: Option<Tournament>,
}

impl Timer {
    /// timers are light weight enough that they just exist until the next time the server bounces
    pub fn get(timer_id: Uuid) -> dashmap::mapref::one::Ref<'static, Uuid, Timer> {
        match TIMERS.get(&timer_id) {
            // the happy path, where no write lock is needed
            Some(x) => x,
            None => {
                let ret = TIMERS
                    .entry(timer_id)
                    .or_insert_with(|| Timer::make_timer(timer_id));
                 ret.downgrade()
            }
        }
    }
    /// timers are light weight enough that they just exist until the next time the server bounces
    pub fn get_mut(timer_id: Uuid) -> dashmap::mapref::one::RefMut<'static, Uuid, Timer> {
        let ret = TIMERS
            .entry(timer_id)
            .or_insert_with(|| Timer::make_timer(timer_id));
        ret
    }

    pub fn for_running_timers<T>(mut f: T)
    where
        T: FnMut(&Timer) -> (),
    {
        TIMERS
            .iter()
            .filter(|timer| {
                if let Some(t) = &timer.tournament {
                    now()
                        .signed_duration_since(t.created)
                        .le(&Duration::weeks(1))
                } else {
                    false
                }
            })
            .for_each(|t| f(&t));
    }

    fn make_timer(timer_id: Uuid) -> Timer {
        let (tx, mut rx) = async_broadcast::broadcast(100);
        let new_timer = Timer {
            timer_id: timer_id.clone(),
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
                            info!("Overflow on channel: {e}")
                        }
                        async_broadcast::RecvError::Closed => {
                            panic!("Timer channel is closed! Should stay open");
                        }
                    },

                    Ok((message, from_device_id)) => {
                        let notification = Arc::new(match &message {
                            TournamentMessage::Started => Notification {
                                title: "Hello".to_string(),
                                body: "Notifications are on".to_string(),
                            },

                            TournamentMessage::Pause => Notification {
                                title: "Update".to_string(),
                                body: "Tournament Paused".to_string(),
                            },
                            TournamentMessage::Resume => Notification {
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
                            TournamentMessage::Settings => Notification {
                                title: "Update".to_string(),
                                body: "Tournament settings have changed".to_string(),
                            },
                            // this doesnt result in a notification
                            TournamentMessage::Ended => Notification {
                                title: "Update".to_string(),
                                body: "Tournament has been terminated".to_string(),
                            },
                            TournamentMessage::OneMinuteWarning => Notification {
                                title: "Update".to_string(),
                                body: "Take your seats! One minute till CIA".to_string(),
                            },
                            // this doesnt result in a notification except for the device that is
                            // turning on the notification
                            TournamentMessage::NotificationChange(device_id) => {
                                if let Some(subscription) = Timer::get(timer_id).subscription(device_id) {
                                    send_notification(subscription, Arc::new(
                                        Notification {
                                            title: "Update".to_string(),
                                            body: "Notifications are on.".to_string(),
                                    })).await;
                                }
                                continue
                            }
                        });
                        let subscriptions = match &Timer::get(timer_id).tournament {
                            Some(tournament) => tournament.subscriptions.clone(),
                            None => continue,
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

    pub fn subscription(&self, device_id : &Uuid) -> Option<&Subscription> {
        match &self.tournament {
            Some(tournament) => tournament.subscriptions.get(device_id),
            None => None,
        }
    }

    pub fn make_tournament(&mut self, structure_name: String) -> Result<(), ServerFnError> {
        if self.tournament.is_none() {
            let tournament = Tournament::new(self, structure_name)?;
            self.tournament = Some(tournament);
            (&*self).broadcast(None, TournamentMessage::Started);
        }
        Ok(())
    }

    pub fn make_tournament_from_storage(
        &mut self,
        storage: StoredTournament,
    ) -> Result<(), ServerFnErrorErr> {
        let tournament = Tournament::from_storage(self, storage)?;
        self.tournament = Some(tournament);
        (&*self).broadcast(None, TournamentMessage::Started);
        Ok(())
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

    pub fn subscribe(&mut self, payload: Subscription) {
        let device_id = payload.device_id;
        match &mut self.tournament {
            Some(ref mut tournament) => {
                tournament.subscriptions.remove(&device_id);
                tournament.subscriptions.insert(device_id, payload);
                info!("Device {} is subscribed.", device_id);
                self.broadcast(None, TournamentMessage::NotificationChange(device_id));
            }
            None => {}
        }
    }

    pub fn unsubscribe(&mut self, payload: Subscription) {
        match &mut self.tournament {
            Some(ref mut tournament) => {
                tournament.subscriptions.remove(&payload.device_id);
                info!("Device {} is unsubscribed.", payload.device_id);
                self.broadcast(
                    None,
                    TournamentMessage::NotificationChange(payload.device_id),
                );
            }
            None => {}
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
                (&*self).broadcast(None, TournamentMessage::Ended);
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
        (&*self).broadcast(None, TournamentMessage::Ended);
    }

    pub fn to_timer_comp_state(&self, device: &Uuid) -> TimerCompState {
        if let Some(tournament) = &self.tournament {
            let subscribed = tournament.subscriptions.contains_key(&device);
            TimerCompState::Running {
                subscribed,
                state: tournament.to_roundstate(),
            }
        } else {
            TimerCompState::NoTournament
        }
    }
    pub fn update_settings(&mut self, duration_override: Option<Duration>) {
        if let Some(ref mut tournament) = &mut self.tournament {
            tournament.update_settings(duration_override);
            (&*self).broadcast(None, TournamentMessage::Settings);
        }
    }

    fn resume_tournament(&mut self, device_id: Uuid) {
        if let Some(ref mut tournament) = &mut self.tournament {
            tournament.clock_state = tournament.clock_state.resume();
            (&*self).broadcast(Some(device_id), TournamentMessage::Resume);
        }
    }

    fn pause_tournament(&mut self, device_id: Uuid) {
        if let Some(ref mut tournament) = &mut self.tournament {
            tournament.clock_state = tournament.clock_state.pause();
            (&*self).broadcast(Some(device_id), TournamentMessage::Pause);
        }
    }
    pub fn execute(&mut self, cmd: &Command, device_id: Uuid) {
        match cmd {
            Command::Resume => {
                self.resume_tournament(device_id);
            }
            Command::Pause => {
                self.pause_tournament(device_id);
            }
            Command::PrevLevel => {
                self.level_up(-1);
            }
            Command::NextLevel => {
                self.level_up(1);
            }
            Command::Terminate => {
                self.terminate();
            }
        }
    }
}

pub struct Tournament {
    pub created: DateTime,
    pub timer_id: Uuid,
    pub structure_name: String,
    structure: Arc<Structure>,
    pub level: usize,
    pub clock_state: ClockState,
    pub duration_override: Option<Duration>,
    /// The devices that have PWA notification active for the current tournament
    pub subscriptions: HashMap<Uuid, Subscription>,
}
// return true if the tournament is complete
enum LevelUpResult {
    Done,
    Invalid,
    Ok,
}

impl Tournament {
    fn from_storage(timer: &Timer, args: StoredTournament) -> Result<Tournament, ServerFnError> {
        let rx = timer.event_sender.new_receiver();
        let timer_id = timer.timer_id;
        let structure = STRUCTURES
            .get(&args.structure_name)
            .ok_or(ServerFnError::new("Structure not found"))?
            .clone();
        let clock = if args.clock_paused {
            ClockState::Paused {
                remaining: args.clock_remaining,
            }
        } else {
            ClockState::Running {
                remaining: args.clock_remaining,
                asof: args.clock_asof,
            }
        };
        let tournament = Tournament {
            created: args.created,
            timer_id: timer.timer_id,
            structure_name: args.structure_name,
            structure: structure.clone(),
            level: args.level,
            clock_state: clock,
            duration_override: args.duration_override,
            subscriptions: args.subscriptions,
        };
        tournament.init(timer_id, rx);
        return Ok(tournament);
    }

    fn new(timer: &Timer, structure_name: String) -> Result<Tournament, ServerFnError> {
        let rx = timer.event_sender.new_receiver();
        let timer_id = timer.timer_id;
        let structure = STRUCTURES
            .get(&structure_name)
            .ok_or(ServerFnError::new("Structure not found"))?
            .clone();
        let clock_state = ClockState::Paused {
            remaining: structure.get_level(1).duration(),
        };
        let tournament = Tournament {
            created: now(),
            timer_id: timer.timer_id,
            structure_name,
            structure,
            level: 1,
            clock_state,
            duration_override: None,
            subscriptions: HashMap::new(),
        };
        tournament.init(timer_id, rx);
        return Ok(tournament);
    }

    fn init(
        &self,
        timer_id: Uuid,
        mut rx: async_broadcast::Receiver<(TournamentMessage, Option<Uuid>)>,
    ) {
        // start a thread to do the level changes
        tokio::spawn(async move {
            // have we given the one minute warning for the break yet
            let mut gave_warning = false;
            loop {
                {
                    let time = match &Timer::get(timer_id).tournament {
                        None => {
                            // the tournament ended
                            break;
                        }
                        Some(tournament) => {
                            let r = tournament.clock_state.remaining();
                            match tournament.structure.get_level(tournament.level) {
                                Level::Break { .. } => {
                                    if !gave_warning {
                                        // wait util < a minute left to give the warning
                                        r.checked_sub(&Duration::seconds(59))
                                            .unwrap_or(Duration::zero())
                                    } else {
                                        r
                                    }
                                }
                                _ => r,
                            }
                        }
                    };
                    // wait until the time has elapsed, or a message that changed the state of the
                    // tournament occurred.
                    tokio::select! {
                        _ = sleep(std::time::Duration::from_millis(time.num_milliseconds() as u64)) => {},
                        _ = rx.recv() => {}
                    }
                }

                {
                    let mut timer = Timer::get_mut(timer_id);
                    match &timer.tournament {
                        None => {
                            // the tournament ended
                            break;
                        }
                        Some(tournament) => {
                            if tournament.clock_state.remaining().num_seconds() < 2 {
                                let done = timer.level_up(1);
                                gave_warning = false;
                                if done {
                                    break;
                                }
                            } else if let Level::Break { .. } =
                                tournament.structure.get_level(tournament.level)
                            {
                                // maybe give one minute warning
                                if !gave_warning
                                    && (tournament.clock_state.remaining() < Duration::minutes(1))
                                {
                                    gave_warning = true;
                                    timer.broadcast(None, TournamentMessage::OneMinuteWarning);
                                }
                            }
                        }
                    }
                }
            }
            info!("Deleting tournament {timer_id}");
        });
    }

    fn level_up(&mut self, delta: i8) -> LevelUpResult {
        if self.level as i8 + delta < 0 {
            return LevelUpResult::Invalid;
        }
        self.level = (self.level as i8 + delta) as usize;
        let level = self.structure.get_level(self.level);
        if level == &Level::Done {
            return LevelUpResult::Done;
        }
        let duration = match self.duration_override {
            Some(duration) => duration,
            None => level.duration(),
        };
        self.clock_state = match self.clock_state {
            ClockState::Paused { .. } => ClockState::Paused {
                remaining: duration,
            },
            ClockState::Running { .. } => ClockState::Running {
                remaining: duration,
                asof: now(),
            },
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
                remaining: *remaining - now().signed_duration_since(asof),
            },
        }
    }
    pub(self) fn resume(&self) -> ClockState {
        match self {
            Self::Paused { remaining } => Self::Running {
                remaining: *remaining,
                asof: now(),
            },
            Self::Running { .. } => *self,
        }
    }
}

pub async fn create_tournament(
    timer_id: Uuid,
    structure_name: String,
) -> Result<(), ServerFnError> {
    // make the timer if it does not exist yet
    let mut timer = Timer::get_mut(timer_id);
    if timer.tournament.is_some() {
        return Ok(());
    }
    info!("Creating tournament {timer_id}");
    timer.make_tournament(structure_name)
}

pub fn tourament_settings(timer_id: Uuid) -> Result<Option<Duration>, ServerFnError> {
    match &Timer::get(timer_id).tournament {
        Some(t) => Ok(t.duration_override),
        None => Err(ServerFnError::new("running tournament")),
    }
}

pub fn set_tournament_settings(
    timer_id: Uuid,
    duration_override: Option<Duration>,
) -> Result<(), ServerFnError> {
    Timer::get_mut(timer_id).update_settings(duration_override);
    Ok(())
}

pub async fn handle_socket(timer_id: Uuid, device_id: Uuid, mut socket: WebSocket) {
    let (mut channel, hello) = {
        let timer = Timer::get(timer_id);
        (
            timer.event_sender.new_receiver(),
            DeviceMessage::NewState(timer.to_timer_comp_state(&device_id)),
        )
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
                    if let TournamentMessage::NotificationChange(changing_device) = tm {
                        // this only changes the state of the device that is changing its subscription
                        if changing_device != device_id {
                            continue;
                        }
                    }
                    if let TournamentMessage::OneMinuteWarning = tm {
                        // this doesn't change the state, only gives a notification elsewhere
                        continue;
                    }
                    if let TournamentMessage::LevelUp(_) = tm {
                        let message = JsonSerdeWasmCodec::encode(&DeviceMessage::Beep).expect("Couldn't encode");
                        if let Err(e) = socket.send(Message::Text(message)).await {
                            info!("couldn't send {e}");
                            break;
                        }
                    }
                    let message = Timer::get(timer_id).to_timer_comp_state(&device_id);
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
                        Ok(cmd) =>  {
                            Timer::get_mut(timer_id).execute( &cmd, device_id);
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
