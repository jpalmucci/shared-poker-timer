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

pub static TIMERS: Lazy<DashMap<Uuid, Timer>> = Lazy::new(|| DashMap::new());

// an internal message that is passed on the backend message bus
#[derive(Clone, serde::Deserialize, serde::Serialize, Debug)]
pub enum TournamentMessage {
    SubscriptionChange(Uuid),
    Hello(RoundState),
    Goodbye,
    Pause(RoundState),
    Resume(RoundState),
    LevelUp(RoundState),
    Settings(RoundState),
}

pub struct Timer {
    pub timer_id: Uuid,
    /// The devices that have PWA notification active for the current tournament
    subscriptions: HashMap<Uuid, Subscription>,
    /// contains the message and the device ID responsible for the message (if there is one)
    /// this is useful if you want to stifle a PWA notification resulting from an action
    /// that user initialted (which would be annoying)
    pub event_sender: async_broadcast::Sender<(TournamentMessage, Option<Uuid>)>,
    /// The currently running tournament (if there is one)
    pub tournament: Option<Tournament>,
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
    fn make_tournament(&mut self, structure_name: String) -> Result<(), ServerFnError> {
        if self.tournament.is_none() {
            let tournament = Tournament::new(self, structure_name)?;
            let message = TournamentMessage::Hello(tournament.to_roundstate());
            self.tournament = Some(tournament);
            (&*self).broadcast(None, message);
        }
        Ok(())
    }

    pub fn make_tournament_from_storage(
        &mut self,
        storage: StoredTournament,
    ) -> Result<(), ServerFnErrorErr> {
        let tournament = Tournament::from_storage(self, storage)?;
        let message = TournamentMessage::Hello(tournament.to_roundstate());
        self.tournament = Some(tournament);
        (&*self).broadcast(None, message);
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
        self.subscriptions.remove(&device_id);
        self.subscriptions.insert(device_id, payload);
        info!("Device {} is subscribed.", device_id);
        self.broadcast(None, TournamentMessage::SubscriptionChange(device_id));
    }

    pub fn unsubscribe(&mut self, payload : Subscription) {
        self.subscriptions.remove(&payload.device_id);
        info!("Device {} is unsubscribed.", payload.device_id);
        self.broadcast(None, TournamentMessage::SubscriptionChange(payload.device_id));
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
                // subscriptions (for pwa notification) expire when the tournament ends
                self.subscriptions.clear();
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
        // subscriptions (for pwa notification) expire when the tournament ends
        self.subscriptions.clear();
        (&*self).broadcast(None, TournamentMessage::Goodbye);
    }

    pub fn to_timer_comp_state(&self, device: &Uuid) -> TimerCompState {
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
    pub created: DateTime,
    pub timer_id: Uuid,
    pub structure_name: String,
    structure: Arc<Structure>,
    pub level: usize,
    pub clock_state: ClockState,
    pub duration_override: Option<Duration>,
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
        timer.make_tournament(structure_name)
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

pub fn execute(cmd: &Command, timer_id: Uuid, device_id: Uuid) {
    match cmd {
        Command::Resume => {
            if let Some(mut timer) = TIMERS.get_mut(&timer_id) {
                timer.resume_tournament(device_id);
            }
        }
        Command::Pause => {
            if let Some(mut timer) = TIMERS.get_mut(&timer_id) {
                timer.pause_tournament(device_id);
            }
        }
        Command::PrevLevel => {
            if let Some(mut timer) = TIMERS.get_mut(&timer_id) {
                timer.level_up(-1);
            }
        }
        Command::NextLevel => {
            if let Some(mut timer) = TIMERS.get_mut(&timer_id) {
                timer.level_up(1);
            }
        }
        Command::Terminate => {
            if let Some(mut timer) = TIMERS.get_mut(&timer_id) {
                timer.terminate();
            }
        }
    }
}

pub async fn handle_socket(timer_id: Uuid, device_id: Uuid, mut socket: WebSocket) {
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
                        Ok(cmd) =>  {
                            execute( &cmd, timer_id, device_id);
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
