use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct PayloadEntry {
    pub index: usize,
    pub data: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PayloadConfig {
    pub payloads: Vec<PayloadEntry>,
}

impl PayloadConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let config: PayloadConfig = serde_yaml::from_str(&contents)?;
        Ok(config)
    }

    pub fn get_payload(&self, index: Option<usize>) -> Option<String> {
        match index {
            Some(idx) => self.payloads.iter().find(|p| p.index == idx).map(|p| p.data.clone()),
            None => {
                let random_idx = fastrand::usize(..self.payloads.len());
                Some(self.payloads[random_idx].data.clone())
            }
        }
    }
}
