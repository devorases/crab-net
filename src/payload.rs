use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadEntry {
    pub index: usize,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadConfig {
    pub payloads: Vec<PayloadEntry>,
    #[serde(skip)]
    current_index: usize,
}

impl PayloadConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let mut config: PayloadConfig = serde_yaml::from_str(&contents)?;
        config.current_index = 0; // Initialize the index counter
        Ok(config)
    }

    pub fn get_payload(&self, index: Option<usize>, random: bool, sequential: bool) -> Option<String> {
        if random {
            let random_idx = fastrand::usize(..self.payloads.len());
            Some(self.payloads[random_idx].data.clone())
        } else if sequential {
            // For sequential mode, we ignore the index parameter
            Some(self.payloads[self.current_index].data.clone())
        } else {
            match index {
                Some(idx) => self.payloads.iter().find(|p| p.index == idx).map(|p| p.data.clone()),
                None => Some(self.payloads[0].data.clone()) // Default to first payload if no index specified
            }
        }
    }
    
    pub fn next_sequential_index(&mut self) {
        self.current_index = (self.current_index + 1) % self.payloads.len();
    }
}
