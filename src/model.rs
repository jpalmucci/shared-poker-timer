use std::fmt;

use chrono::Duration;
type DateTime = chrono::DateTime<chrono::Local>;

use serde::{de::Visitor, ser::SerializeStruct, Deserialize, Deserializer, Serialize};
use uuid::Uuid;

#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize, Debug)]
pub enum Level {
    Blinds {
        game: String,
        small: u32,
        big: u32,
        ante: u32,
        duration: chrono::Duration,
    },
    Limit {
        game: String,
        small: u32,
        big: u32,
        duration: chrono::Duration,
    },
    Stud {
        game: String,
        ante: u32,
        bring_in: u32,
        small: u32,
        big: u32,
        duration: chrono::Duration,
    },
    Break {
        duration: chrono::Duration,
    },
    Done,
}

impl Level {
    pub fn duration(&self) -> chrono::Duration {
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
            Self::Break { .. } => "BREAK",
            Self::Done => "FINISHED",
        }
    }

    pub fn make_display_string(&self) -> String {
        match self {
            Level::Blinds {
                small, big, ante, ..
            } => format!["{small} / {big} / {ante}"],
            Level::Limit { small, big, .. } => format!["{small} / {big}  Big Bet: {}", big * 2],
            Level::Break { duration } => format!["{duration} BREAK"],
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

// when sending a clock state across the wire, don't send a DateTime as it will
// screw up when there is time scew between the client and the server
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
                let remaining_asof_now =
                    *remaining - (chrono::Local::now().signed_duration_since(asof));
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
                        asof: chrono::Local::now(),
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
            Self::Running { remaining, asof } => {
                *remaining - chrono::Local::now().signed_duration_since(asof)
            }
        }
    }
}

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

// a message sent from the backend to the app
#[derive(Clone, serde::Deserialize, serde::Serialize, Debug)]
pub enum DeviceMessage {
    NewState(TimerCompState),
    Beep,
}
