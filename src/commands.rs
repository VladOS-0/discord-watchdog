pub mod config;
pub mod misc;

use crate::{Data, Error};

pub fn get_commands() -> Vec<poise::Command<Data, Error>> {
    vec![config::config(), misc::info(), misc::debug()]
}
