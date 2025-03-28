use std::{collections::HashMap, sync::Arc};

use once_cell::sync::Lazy;

use crate::model::*;

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

pub static STRUCTURES: Lazy<HashMap<String, Arc<Structure>>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert(
        "Nightly TOC".to_string(),
        Arc::new(Structure {
            levels: vec![
                Level::Limit {
                    game: "Hold Em".to_string(),
                    small: 200,
                    big: 400,
                    duration: Duration::minutes(20),
                },
                Level::Limit {
                    game: "Omaha Hi/Lo".to_string(),
                    small: 200,
                    big: 500,
                    duration: Duration::minutes(20),
                },
                Level::Stud {
                    game: "Stud Hi/Lo".to_string(),
                    ante: 100,
                    bring_in: 200,
                    small: 600,
                    big: 1200,
                    duration: Duration::minutes(20),
                },
                Level::Limit {
                    game: "Hold Em".to_string(),
                    small: 400,
                    big: 800,
                    duration: Duration::minutes(20),
                },
                Level::Limit {
                    game: "Omaha Hi/Lo".to_string(),
                    small: 500,
                    big: 1000,
                    duration: Duration::minutes(20),
                },
                Level::Stud {
                    game: "Stud Hi/Lo".to_string(),
                    ante: 300,
                    bring_in: 400,
                    small: 1200,
                    big: 2400,
                    duration: Duration::minutes(20),
                },
                Level::Break {
                    duration: Duration::minutes(10),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 500,
                    big: 1000,
                    ante: 1000,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 600,
                    big: 1200,
                    ante: 1200,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 1000,
                    big: 1500,
                    ante: 1500,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 1000,
                    big: 2000,
                    ante: 2000,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 1500,
                    big: 2500,
                    ante: 2500,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 1500,
                    big: 3000,
                    ante: 3000,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 2000,
                    big: 4000,
                    ante: 4000,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 2500,
                    big: 5000,
                    ante: 5000,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 3000,
                    big: 6000,
                    ante: 6000,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 4000,
                    big: 8000,
                    ante: 8000,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 5000,
                    big: 10000,
                    ante: 10000,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 6000,
                    big: 12000,
                    ante: 12000,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 8000,
                    big: 16000,
                    ante: 16000,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 10000,
                    big: 20000,
                    ante: 20000,
                    duration: Duration::minutes(20),
                },
            ],
        }),
    );

    map.insert(
        "Nightly NLHE".to_string(),
        Arc::new(Structure {
            levels: vec![
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 100,
                    big: 200,
                    ante: 200,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 200,
                    big: 300,
                    ante: 300,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 200,
                    big: 400,
                    ante: 400,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 300,
                    big: 500,
                    ante: 500,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 300,
                    big: 600,
                    ante: 600,
                    duration: Duration::minutes(20),
                },
                Level::Break {
                    duration: Duration::minutes(10),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 400,
                    big: 800,
                    ante: 800,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 500,
                    big: 1000,
                    ante: 1000,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 600,
                    big: 1200,
                    ante: 1200,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 1000,
                    big: 1500,
                    ante: 1500,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 1000,
                    big: 2000,
                    ante: 2000,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 1500,
                    big: 2500,
                    ante: 2500,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 1500,
                    big: 3000,
                    ante: 3000,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 2000,
                    big: 4000,
                    ante: 4000,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 2500,
                    big: 5000,
                    ante: 5000,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 3000,
                    big: 6000,
                    ante: 6000,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 4000,
                    big: 8000,
                    ante: 8000,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 5000,
                    big: 10000,
                    ante: 10000,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 6000,
                    big: 12000,
                    ante: 12000,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 10000,
                    big: 15000,
                    ante: 15000,
                    duration: Duration::minutes(20),
                },
                Level::Blinds {
                    game: "NLHE".to_string(),
                    small: 10000,
                    big: 20000,
                    ante: 20000,
                    duration: Duration::minutes(20),
                },
            ],
        }),
    );

    map
});
