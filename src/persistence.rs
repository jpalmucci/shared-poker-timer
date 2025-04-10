//! Dont use any kind of back end database to keep things lightweight.
//! However, when updating the server code the container needs to bounce.
//! This module supports reloading the currently running poker timers
//! when the new server comes up so we don't interrupt any games.

use std::{
    collections::HashMap,
    fs,
    io::{BufReader, Write},
};

use log::error;
use uuid::Uuid;

use crate::{
    backend::Subscription,
    model::*,
    timers::{Timer, Tournament},
};

/// This is the format that the tournaments are stored on disk.
/// It must remain bacward compatible to the previously running version
#[derive(serde::Serialize, serde::Deserialize)]
pub struct StoredTournament {
    pub timer_id: Uuid,
    pub created: DateTime,
    pub structure_name: String,
    pub level: usize,
    // we cannot use the clockstate serialization here because it is designed to
    // go over near instantaineous channels
    pub clock_paused: bool,
    pub clock_remaining: Duration,
    pub clock_asof: DateTime,
    pub duration_override: Option<Duration>,
    pub subscriptions: HashMap<Uuid, Subscription>,
}

impl From<&Tournament> for StoredTournament {
    fn from(value: &Tournament) -> Self {
        StoredTournament {
            timer_id: value.timer_id,
            created: value.created,
            structure_name: value.structure_name.clone(),
            level: value.level,
            clock_paused: value.clock_state.is_paused(),
            clock_remaining: value.clock_state.remaining(),
            clock_asof: now(),
            duration_override: value.duration_override,
            subscriptions: value.subscriptions.clone(),
        }
    }
}

/// Save the running tournaments that a less than a week old
pub fn save_running() -> Result<(), Box<dyn std::error::Error>> {
    let mut timers: Vec<StoredTournament> = vec![];
    Timer::for_running_timers(|t| {
        timers.push(StoredTournament::from(t.tournament.as_ref().unwrap()))
    });

    if timers.len() > 0 {
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open("./storage/timers.json")?
            .write_all(&serde_json::to_vec(&timers)?)?;
    }
    Ok(())
}

pub fn load_saved() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::Path::new("./storage/timers.json");
    if !path.exists() {
        return Ok(());
    }
    let tournaments =
        serde_json::from_reader::<_, Vec<StoredTournament>>(BufReader::new(fs::File::open(path)?));
    match tournaments {
        Err(e) => {
            error!("bad timers.json file, punting: {e}");
        }
        Ok(tournaments) => {
            for t in tournaments.into_iter() {
                let timer_id = t.timer_id;
                let mut timer = Timer::get_mut(timer_id);
                timer.make_tournament_from_storage(t)?;
            }

        }
    }
    let backpath = std::path::Path::new("./storage/timers.json.backup");
    if backpath.exists() {
        fs::remove_file(backpath)?;
    }
    fs::rename(path, backpath)?;

    Ok(())
}
