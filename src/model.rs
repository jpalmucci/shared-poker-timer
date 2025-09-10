//! All the data that is sent between the back end and the front end

use std::fmt;

pub type Duration = chrono::Duration;
pub type DateTime = chrono::DateTime<chrono::Local>;
pub fn now() -> DateTime {
    chrono::Local::now()
}

use serde::{Deserialize, Deserializer, Serialize, de::Visitor, ser::SerializeStruct};
use uuid::Uuid;

#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize, Debug)]
pub enum Level {
    Blinds {
        game: String,
        small: u32,
        big: u32,
        ante: Option<u32>,
        duration: Duration,
    },
    Limit {
        game: String,
        small: u32,
        big: u32,
        duration: Duration,
    },
    Stud {
        game: String,
        ante: u32,
        bring_in: u32,
        small: u32,
        big: u32,
        duration: Duration,
    },
    Break {
        duration: Duration,
    },
    Done,
}

impl Level {
    pub fn duration(&self) -> Duration {
        match self {
            Self::Blinds { duration, .. } => duration.clone(),
            Self::Limit { duration, .. } => duration.clone(),
            Self::Stud { duration, .. } => duration.clone(),
            Self::Break { duration, .. } => duration.clone(),
            Self::Done => Duration::seconds(0),
        }
    }
    pub fn game(&self) -> &str {
        match self {
            Self::Blinds { game, .. } => game,
            Self::Limit { game, .. } => game,
            Self::Stud { game, .. } => game,
            Self::Break { .. } => "",
            Self::Done => "FINISHED",
        }
    }

    // the string that we display in "next level" and in level up notifications
    pub fn short_level_string(&self) -> String {
        match self {
            Level::Blinds {
                game,
                small,
                big,
                ante,
                ..
            } => {
                if let Some(ante) = ante {
                    format!["{game} {small} / {big} / {ante}"]
                } else {
                    format!["{game} {small} / {big}"]
                }
            }
            Level::Limit {
                game, small, big, ..
            } => format!["{game} {small} / {big}  Big Bet: {}", big * 2],
            Level::Break { duration } => {
                let min = duration.num_minutes();
                format!["{min} MINUTE BREAK"]
            }
            Level::Done => "FINISHED".to_string(),
            Level::Stud {
                ante,
                bring_in,
                small,
                big,
                game,
                ..
            } => format!("{game} Ante: {ante} Bring: {bring_in} Small: {small} Big: {big}"),
        }
    }

    // the string that we display on the main timer (the game is displayed above it)
    pub fn make_level_string(&self) -> String {
        match self {
            Level::Blinds {
                small, big, ante, ..
            } => {
                if let Some(ante) = ante {
                    format!["{small} / {big} / {ante}"]
                } else {
                    format!["{small} / {big}"]
                }
            }
            Level::Limit { small, big, .. } => format!["{small} / {big}  Big Bet: {}", big * 2],
            Level::Break { duration } => {
                let min = duration.num_minutes();
                format!["{min} MINUTE BREAK"]
            }
            Level::Done => "FINISHED".to_string(),
            Level::Stud {
                ante,
                bring_in,
                small,
                big,
                ..
            } => format!("Ante: {ante} Bring: {bring_in} Small: {small} Big: {big}"),
        }
    }
}

/// The state of a clock. Can be pause or running, each with some duration left
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum ClockState {
    Paused { remaining: Duration },
    Running { remaining: Duration, asof: DateTime },
}

impl fmt::Display for ClockState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut r = self.remaining();
        if r < Duration::zero() {
            r = Duration::zero();
        }
        let minutes = r.num_seconds() / 60;
        let seconds = r.num_seconds() % 60;
        write!(f, "{:02}:{:02}", minutes, seconds)?;
        Ok(())
    }
}

/// when sending a clock state across the wire, don't send a DateTime as it will
/// screw up when there is time scew between the client and the server
/// just send the duration, and decode it using the current machine time at the end
/// (we assume that the network transit time is negligable)
impl Serialize for ClockState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ClockState::Paused { remaining } => {
                let mut s = serializer.serialize_struct("ClockState", 2)?;
                s.serialize_field("paused", &true)?;
                s.serialize_field("remaining", remaining)?;
                s.end()
            }
            ClockState::Running { remaining, asof } => {
                let mut s = serializer.serialize_struct("ClockState", 2)?;
                s.serialize_field("paused", &false)?;
                let remaining_asof_now = *remaining - (now().signed_duration_since(asof));
                s.serialize_field("remaining", &remaining_asof_now)?;
                s.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for ClockState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ClockVisitor;
        impl<'de> Visitor<'de> for ClockVisitor {
            type Value = ClockState;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("A clock state consisting of a paused and duration field")
            }
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut paused: Option<bool> = None;
                let mut remaining: Option<Duration> = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "paused" => {
                            paused = Some(map.next_value()?);
                        }
                        "remaining" => {
                            remaining = Some(map.next_value()?);
                        }
                        _ => {}
                    }
                }
                if paused.unwrap() {
                    Ok(ClockState::Paused {
                        remaining: remaining.unwrap(),
                    })
                } else {
                    Ok(ClockState::Running {
                        remaining: remaining.unwrap(),
                        asof: now(),
                    })
                }
            }
        }
        deserializer.deserialize_struct("ClockState", &["paused", "duration"], ClockVisitor)
    }
}

impl ClockState {
    pub fn is_paused(&self) -> bool {
        match self {
            Self::Paused { .. } => true,
            Self::Running { .. } => false,
        }
    }
    pub fn remaining(&self) -> Duration {
        match self {
            Self::Paused { remaining } => *remaining,
            Self::Running { remaining, asof } => *remaining - now().signed_duration_since(asof),
        }
    }
}

/// All the information that you need to display the current level in the browser
#[derive(PartialEq, Clone, serde::Deserialize, serde::Serialize, Debug)]
pub struct RoundState {
    pub cur: Level,
    pub next: Level,
    pub timer_id: Uuid,
    pub level: usize,
    pub clock: ClockState,
}

/// The state of the timer component
#[derive(serde::Deserialize, serde::Serialize, Clone, PartialEq, Debug)]
pub enum TimerCompState {
    Loading,
    NoTournament,
    Running { subscribed: bool, state: RoundState },
    Error(String),
}

/// a message sent from the backend to the app
#[derive(Clone, serde::Deserialize, serde::Serialize, Debug)]
pub enum DeviceMessage {
    NewState(TimerCompState),
    Beep,
}

/// a message sent from the app to the backend
#[derive(Clone, serde::Deserialize, serde::Serialize, Debug)]
pub enum Command {
    Pause,
    Resume,
    NextLevel,
    PrevLevel,
    Terminate,
}
