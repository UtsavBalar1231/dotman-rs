pub mod config;
pub mod errors;
mod hasher;
mod file_manager;
use ron::{extensions::Extensions, ser::PrettyConfig};

pub fn get_ron_formatter() -> PrettyConfig {
    PrettyConfig::new()
        .depth_limit(2)
        .extensions(Extensions::IMPLICIT_SOME)
}

