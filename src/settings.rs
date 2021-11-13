#![allow(dead_code)]
use crate::Result;
use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "kebab-case")]
pub enum ConfigCommand {
    /// Set a config value
    Set {
        #[structopt(help = "Setting to set")]
        setting: String,
        #[structopt(help = "Setting value")]
        value: String,
    },
    /// Get a config value
    Get {
        #[structopt(help = "Setting to get")]
        setting: String,
    },
    /// List all Hyrule Builder settings
    List {
        #[structopt(long, short, help = "Show setting values")]
        values: bool,
    },
    /// Import settings from other programs
    Import {
        #[structopt(
            long,
            short = "b",
            required_unless = "from-cemu",
            help = "Set game folders from BCML settings"
        )]
        from_bcml: bool,
        #[structopt(
            long,
            short = "c",
            required_unless = "from-bcml",
            requires = "cemu-dir",
            help = "(Wii U) Set game folders from Cemu folder"
        )]
        from_cemu: bool,
        #[structopt(help = "Cemu directory (or MLC folder if separate)")]
        cemu_dir: Option<String>,
    },
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Settings {
    pub game_dir: Option<PathBuf>,
    pub game_dir_nx: Option<PathBuf>,
    pub update_dir: Option<PathBuf>,
    pub dlc_dir: Option<PathBuf>,
    pub dlc_dir_nx: Option<PathBuf>,
}

fn print_setting(setting: &Option<PathBuf>) -> &str {
    setting
        .as_ref()
        .map(|d| d.to_str().unwrap())
        .unwrap_or("-Not set-")
}

impl Settings {
    #[inline]
    pub fn get_settings_path() -> PathBuf {
        dirs2::data_local_dir()
            .context("Failed to get local data dir")
            .unwrap()
            .join("hyrule_builder/settings.yml")
    }

    pub fn list(&self, values: bool) {
        println!("Hyrule Builder configuration settings:");
        println!(
            "  game_dir:     {}",
            if values {
                print_setting(&self.game_dir)
            } else {
                "Wii U base game folder"
            }
        );
        println!(
            "  update_dir:   {}",
            if values {
                print_setting(&self.update_dir)
            } else {
                "Wii U update folder"
            }
        );
        println!(
            "  dlc_dir:      {}",
            if values {
                print_setting(&self.dlc_dir)
            } else {
                "Wii U DLC folder"
            }
        );
        println!(
            "  game_dir_nx:  {}",
            if values {
                print_setting(&self.game_dir_nx)
            } else {
                "Switch base game folder"
            }
        );
        println!(
            "  dlc_dir_nx:   {}",
            if values {
                print_setting(&self.dlc_dir_nx)
            } else {
                "Switch DLC folder"
            }
        );
    }

    pub fn get(&self, setting: &str) -> Result<()> {
        println!(
            "{}",
            match setting {
                "game_dir" => print_setting(&self.game_dir),
                "update_dir" => print_setting(&self.update_dir),
                "dlc_dir" => print_setting(&self.dlc_dir),
                "game_dir_nx" => print_setting(&self.game_dir_nx),
                "dlc_dir_nx" => print_setting(&self.dlc_dir_nx),
                _ => return Err(anyhow!("Error: Invalid setting")),
            }
        );
        Ok(())
    }

    pub fn set(&mut self, setting: &str, value: &str) -> Result<()> {
        let path = PathBuf::from(value);
        if !path.exists() {
            Err(anyhow!("{} does not exist", path.display()))
        } else {
            match setting {
                "game_dir" => self.game_dir = Some(path),
                "update_dir" => self.update_dir = Some(path),
                "dlc_dir" => self.dlc_dir = Some(path),
                "game_dir_nx" => self.game_dir_nx = Some(path),
                "dlc_dir_nx" => self.dlc_dir_nx = Some(path),
                _ => {
                    return Err(anyhow!("Invalid setting"));
                }
            };
            self.save()?;
            Ok(())
        }
    }

    pub fn set_from_cemu(&mut self, cemu_dir: &str) -> Result<()> {
        let cemu_dir = PathBuf::from(cemu_dir);
        if !cemu_dir.exists() {
            Err(anyhow::anyhow!("Specified Cemu directory does not exist"))
        } else {
            println!("Detecting base game folder...");
            let needle = glob::glob(
                cemu_dir
                    .join("mlc01")
                    .join("**/Dungeon001.pack")
                    .to_str()
                    .unwrap(),
            )?
            .find_map(Result::ok)
            .ok_or_else(|| anyhow!("Base game directory not found in Cemu MLC"))?;
            self.game_dir = Some(
                needle
                    .parent()
                    .context("No Pack folder")?
                    .parent()
                    .context("No content folder")?
                    .to_path_buf(),
            );
            println!("Found: {}", self.game_dir.as_ref().unwrap().display());
            println!("Detecting update folder...");
            let needle = glob::glob(
                cemu_dir
                    .join("mlc01")
                    .join("**/Enemy_Lynel_Senior.sbactorpack")
                    .to_str()
                    .unwrap(),
            )?
            .find_map(Result::ok)
            .ok_or_else(|| anyhow!("Update directory not found in Cemu MLC"))?;
            self.update_dir = Some(
                needle
                    .parent()
                    .context("No Actor/Pack folder")?
                    .parent()
                    .context("No Actor folder")?
                    .parent()
                    .context("No content folder")?
                    .to_path_buf(),
            );
            println!("Found: {}", self.update_dir.as_ref().unwrap().display());
            println!("Detecting DLC folder...");
            let needle = glob::glob(
                cemu_dir
                    .join("mlc01")
                    .join("**/AocMainField.pack")
                    .to_str()
                    .unwrap(),
            )?
            .find_map(Result::ok)
            .ok_or_else(|| anyhow!("DLC directory not found in Cemu MLC"))?;
            self.dlc_dir = Some(
                needle
                    .parent()
                    .context("No Pack folder")?
                    .parent()
                    .context("No content folder")?
                    .to_path_buf(),
            );
            println!("Found: {}", self.dlc_dir.as_ref().unwrap().display());
            println!("Game folders set successfully");
            self.save()?;
            Ok(())
        }
    }

    pub fn set_from_bcml(&mut self) -> Result<()> {
        println!("Loading BCML settings...");
        let bcml_path = dirs2::data_local_dir().unwrap().join("bcml/settings.json");
        let settings: serde_json::Value = serde_json::from_reader(
            std::fs::File::open(&bcml_path).context("Missing BCML settings file")?,
        )?;
        let settings_map = settings.as_object().context("Invalid BCML settings")?;
        self.game_dir = settings_map
            .get("game_dir")
            .and_then(|d| d.as_str())
            .map(PathBuf::from);
        self.update_dir = settings_map
            .get("update_dir")
            .and_then(|d| d.as_str())
            .map(PathBuf::from);
        self.dlc_dir = settings_map
            .get("dlc_dir")
            .and_then(|d| d.as_str())
            .map(PathBuf::from);
        self.game_dir_nx = settings_map
            .get("game_dir_nx")
            .and_then(|d| d.as_str())
            .map(PathBuf::from);
        self.dlc_dir_nx = settings_map
            .get("dlc_dir_nx")
            .and_then(|d| d.as_str())
            .map(PathBuf::from);
        println!("BCML settings imported successfully.");
        self.list(true);
        Ok(())
    }

    pub fn get_settings() -> Result<Self> {
        let path = Self::get_settings_path();
        if !path.exists() {
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
