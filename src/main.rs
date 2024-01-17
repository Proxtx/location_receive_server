use serde::de::{DeserializeOwned, IntoDeserializer};

mod error;

use {
    error::{FileError, FileResult},
    serde::Deserialize,
    std::{
        fs::File,
        path::{Path, PathBuf},
        time::Duration,
    },
    tokio::{
        fs::{self, File as TokioFile},
        io::AsyncReadExt,
    },
};

fn main() {
    rocket::build().manage(LocationWriter::new(directory, duration));
}

type LocationFile = Vec<(String, LocationSnapshot)>;
type LocationsSnapshot = Vec<(String, LocationSnapshot)>;

#[derive(Deserialize)]
struct LocationSnapshot {
    latitude: String,
    longitude: String,
    address: String,
}
struct LocationWriter<'a> {
    file: ManagedFile<'a>,
}

impl<'a> LocationWriter<'a> {
    pub fn new(directory: &'a Path, duration: Duration) -> LocationWriter {
        LocationWriter {
            file: ManagedFile::new(directory, duration),
        }
    }
}

struct ManagedFile<'a> {
    directory: &'a Path,
    duration: Duration,
}

impl<'a> ManagedFile<'a> {
    pub fn new(directory: &'a Path, duration: Duration) -> ManagedFile {
        ManagedFile {
            directory,
            duration,
        }
    }

    async fn read_latest_file<T>(&mut self) -> FileResult<Option<T>>
    where
        T: DeserializeOwned,
    {
        let newest_file = self.get_newest_file().await?;
        let mut content = String::new();
        Ok(match newest_file {
            Some(newest_file) => {
                TokioFile::open(newest_file)
                    .await?
                    .read_to_string(&mut content)
                    .await?;
                Some(serde_json::from_str::<T>(&content)?)
            }
            None => None,
        })
    }

    async fn get_newest_file(&mut self) -> FileResult<Option<PathBuf>> {
        let mut newest = None;
        let mut location_files = fs::read_dir(self.directory).await?;
        while let Some(loc_file) = location_files.next_entry().await? {
            match loc_file.file_name().into_string()?.split(".").next() {
                Some(file_name) => match file_name.parse::<u64>() {
                    Ok(number) => match newest {
                        Some(current_number) => {
                            if current_number < number {
                                newest = Some(number);
                            }
                        }
                        None => {
                            newest = Some(number);
                        }
                    },
                    _ => {}
                },
                _ => {}
            }
        }

        Ok(match newest {
            Some(v) => {
                let mut path = PathBuf::from(self.directory);
                path.set_file_name(format!("{}.json", v));
                Some(path)
            }
            None => None,
        })
    }
}
