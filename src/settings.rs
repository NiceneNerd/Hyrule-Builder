#![allow(dead_code)]
use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Settings {
    pub game_dir: PathBuf,
    pub game_dir_nx: PathBuf,
    pub update_dir: PathBuf,
    pub dlc_dir: PathBuf,
    pub dlc_dir_nx: PathBuf,
}

impl Settings {
    #[inline]
    pub fn get_settings_path() -> PathBuf {
        dirs2::data_local_dir()
            .unwrap()
            .join("hyrule_builder/settings.yml")
    }

    pub fn get_settings() -> Result<Self> {
        let path = Self::get_settings_path();
        if path.exists() {
            let settings = Self::default();
            std::fs::create_dir_all(path.parent().unwrap())?;
            std::fs::write(path, serde_yaml::to_string(&settings)?)?;
            Ok(settings)
        } else {
            Ok(serde_yaml::from_reader(&std::fs::File::open(path)?)?)
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::get_settings_path();
        if !path.exists() {
            std::fs::create_dir_all(path.parent().unwrap())?;
        }
        Ok(serde_yaml::to_writer(&std::fs::File::create(path)?, self)?)
    }
}
