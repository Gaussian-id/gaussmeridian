use super::MoaConfig;
use crate::MoaResult;
use config::{Config, ConfigError, File};
use std::path::Path;

impl MoaConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> MoaResult<Self> {
        let settings = Config::builder()
            .add_source(File::from(path.as_ref()))
            .build()?;
        
        Ok(settings.try_deserialize()?)
    }
    
    pub fn from_env() -> MoaResult<Self> {
        let settings = Config::builder()
            .add_source(config::Environment::with_prefix("MOA"))
            .build()?;
        
        Ok(settings.try_deserialize()?)
    }

    pub fn load_settings(path: impl AsRef<Path>) -> MoaResult<Self> {
        let settings = config::Config::builder()
            .add_source(config::File::from(path.as_ref()))
            .build()
            .map_err(|err| crate::MoaError::config(err.to_string(), Some(err)))?;

        Ok(settings.try_deserialize()?)
    }
}

impl From<ConfigError> for crate::MoaError {
    fn from(err: ConfigError) -> Self {
        crate::MoaError::config(err.to_string(), Some(err))
    }
}