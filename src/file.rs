use std::time::UNIX_EPOCH;

use serde::Serialize;
use tokio::io::AsyncWriteExt;

use {
    crate::{config::Place, error::FileResult},
    serde::de::DeserializeOwned,
    serde::Deserialize,
    std::{
        collections::HashMap,
        path::{Path, PathBuf},
        time::{Duration, SystemTime},
    },
    tokio::{
        fs::{self, File as TokioFile, OpenOptions},
        io::AsyncReadExt,
    },
};

pub trait InitializeFile {
    fn init() -> Self;
}

trait TimedFile<T> {
    fn get_latest_data(&self) -> FileResult<Option<&T>>;
    fn received_new_data(&mut self, data: T);
}

type LocationFile = HashMap<String, LocationsSnapshot>;
impl InitializeFile for LocationFile {
    fn init() -> Self {
        HashMap::new()
    }
}

impl TimedFile<LocationsSnapshot> for LocationFile {
    fn get_latest_data(&self) -> FileResult<Option<&LocationsSnapshot>> {
        let mut biggest: Option<u64> = None;
        for key in self.keys() {
            match biggest {
                None => {
                    biggest = Some(key.parse()?);
                }
                Some(v) => {
                    let key = key.parse::<u64>()?;
                    if key > v {
                        biggest = Some(key);
                    }
                }
            }
        }

        Ok(match biggest {
            None => None,
            Some(k) => Some(self.get(&k.to_string()).unwrap()),
        })
    }

    fn received_new_data(&mut self, received_data: LocationsSnapshot) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string();
        self.insert(current_time, received_data);
    }
}

type LocationsSnapshot = HashMap<String, LocationSnapshot>;
impl InitializeFile for LocationsSnapshot {
    fn init() -> Self {
        HashMap::new()
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct LocationSnapshot {
    latitude: String,
    longitude: String,
    address: Option<String>,
}

impl LocationSnapshot {
    pub fn new(lat: f64, long: f64, place: Option<Place>) -> Self {
        let converted_address = match place {
            Some(v) => Some(v.name),
            None => None,
        };
        LocationSnapshot {
            address: converted_address,
            latitude: lat.to_string(),
            longitude: long.to_string(),
        }
    }
}
pub struct LocationWriter<'a> {
    file: ManagedDirectory<'a>,
}

impl<'a> LocationWriter<'a> {
    pub fn new(directory: &'a Path, duration: Duration) -> LocationWriter {
        LocationWriter {
            file: ManagedDirectory::new(directory, duration),
        }
    }

    pub async fn location_update(
        &self,
        user_id: String,
        location_snapshot: LocationSnapshot,
    ) -> FileResult<()> {
        let newest_file = self.file.read_latest_file::<LocationFile>().await?;
        let mut current_file = self.file.read_current_file::<LocationFile>().await?;
        let mut updated_data: LocationsSnapshot;
        match newest_file {
            Some((_, newest_file)) => match newest_file.get_latest_data()? {
                Some(newest_data) => {
                    updated_data = newest_data.clone();
                    updated_data.insert(user_id, location_snapshot);
                }
                None => {
                    updated_data = LocationsSnapshot::init();
                    updated_data.insert(user_id, location_snapshot);
                }
            },
            None => {
                updated_data = LocationsSnapshot::init();
                updated_data.insert(user_id, location_snapshot);
            }
        }

        current_file.1.insert(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
                .to_string(),
            updated_data,
        );

        self.file.write_file(current_file.0, current_file.1).await
    }
}

pub struct ManagedDirectory<'a> {
    directory: &'a Path,
    duration: Duration,
}

impl<'a> ManagedDirectory<'a> {
    pub fn new(directory: &'a Path, duration: Duration) -> ManagedDirectory {
        ManagedDirectory {
            directory,
            duration,
        }
    }

    pub async fn read_latest_file<T>(&self) -> FileResult<Option<(u64, T)>>
    where
        T: DeserializeOwned,
    {
        let newest_file = self.get_newest_file().await?;
        Ok(match newest_file {
            Some((file_time, path)) => Some((file_time, ManagedDirectory::read_file(path).await?)),
            None => None,
        })
    }

    async fn read_file<T>(path: PathBuf) -> FileResult<T>
    where
        T: DeserializeOwned,
    {
        let mut content = String::new();
        TokioFile::open(path)
            .await?
            .read_to_string(&mut content)
            .await?;
        Ok(serde_json::from_str(&content)?)
    }

    pub async fn read_current_file<T>(&self) -> FileResult<(u64, T)>
    where
        T: DeserializeOwned + InitializeFile,
    {
        let newest_file = self.get_newest_file().await?;
        match newest_file {
            Some((time, path)) => {
                if (SystemTime::now().duration_since(UNIX_EPOCH).unwrap()
                    - Duration::from_millis(time))
                    < self.duration
                {
                    Ok((time, ManagedDirectory::read_file(path).await?))
                } else {
                    Ok((
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64,
                        T::init(),
                    ))
                }
            }
            None => Ok((
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
                T::init(),
            )),
        }
    }

    async fn get_newest_file(&self) -> FileResult<Option<(u64, PathBuf)>> {
        let mut newest = None;
        let mut location_files = fs::read_dir(self.directory).await?;
        while let Some(loc_file) = location_files.next_entry().await? {
            match loc_file.file_name().into_string()?.split(".").next() {
                Some(file_name) => match file_name.parse::<u64>() {
                    Ok(number) => match newest {
                        Some((current_number, _)) => {
                            if current_number < number {
                                newest = Some((number, loc_file));
                            }
                        }
                        None => {
                            newest = Some((number, loc_file));
                        }
                    },
                    _ => {}
                },
                _ => {}
            }
        }

        Ok(match newest {
            Some((time, dir_entry)) => Some((time, dir_entry.path())),
            None => None,
        })
    }

    async fn write_file<T>(&self, time: u64, data: T) -> FileResult<()>
    where
        T: Serialize,
    {
        let mut new_path = PathBuf::from(self.directory);
        new_path = new_path.join(format!("{}.json", time));
        let mut file = TokioFile::create(new_path).await?;
        file.write_all(serde_json::to_string(&data)?.as_bytes())
            .await?;
        Ok(())
    }
}
