use std::{error::Error, fmt};

pub type FileResult<T> = Result<T, FileError>;

#[derive(Debug)]
pub enum FileError {
    DeserializationError(serde_json::Error),
    IoError(std::io::Error),
    FileNameError(std::ffi::OsString),
}

impl Error for FileError {}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileError::DeserializationError(v) => {
                write!(f, "Was unable to Deserialize file: {}", v)
            }
            FileError::IoError(v) => {
                write!(f, "Was unable to read the file due to an io error: {}", v)
            }
            FileError::FileNameError(v) => {
                write!(f, "Was unable to read the os string: {:?}", v)
            }
        }
    }
}

impl From<serde_json::Error> for FileError {
    fn from(value: serde_json::Error) -> Self {
        FileError::DeserializationError(value)
    }
}

impl From<std::io::Error> for FileError {
    fn from(value: std::io::Error) -> Self {
        FileError::IoError(value)
    }
}

impl From<std::ffi::OsString> for FileError {
    fn from(value: std::ffi::OsString) -> Self {
        FileError::FileNameError(value)
    }
}
