use {
    crate::error::ConfigResult,
    serde::Deserialize,
    std::{collections::HashMap, path::PathBuf},
    tokio::{fs::File, io::AsyncReadExt},
};

#[derive(Deserialize, Clone)]
pub struct Place {
    pub lat: f64,
    pub long: f64,
    pub radius: u8,
    pub name: Option<String>,
}

#[derive(Deserialize)]
pub struct Config {
    pub password: String,
    pub port: u16,
    pub file_duration: u64,
    pub places: HashMap<String, Place>,
    pub file_locations: FileLocations,
    pub users: HashMap<String, User>,
    pub hooks: Option<Vec<Hook>>,
}

#[derive(Deserialize)]
pub struct Hook {
    place: String,
    command: String,
}

#[derive(Deserialize)]
pub struct User {
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
}

#[derive(Deserialize)]
pub struct FileLocations {
    pub location: PathBuf,
    pub data: PathBuf,
}

impl Config {
    pub async fn load() -> ConfigResult<Self> {
        let mut config = String::new();
        File::open("config.toml")
            .await?
            .read_to_string(&mut config)
            .await?;
        Ok(toml::from_str::<Config>(&config)?)
    }
}
