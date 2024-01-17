use tokio::io::AsyncReadExt;

use {crate::error::ConfigResult, serde::Deserialize, std::path::PathBuf, tokio::fs::File};

#[derive(Deserialize, Clone)]
pub struct Place {
    pub lat: f64,
    pub long: f64,
    pub radius: u8,
    pub name: String,
}

#[derive(Deserialize)]
pub struct Config {
    pub password: String,
    pub places: Vec<Place>,
    pub file_locations: FileLocations,
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
