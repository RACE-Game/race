//! The blacklist of game addresses.  We use this list to track
//! malformed games.  Game on this list will not be loaded.  The
//! content will be stored in file `.blacklist` in the working
//! directory.

use std::{fs::OpenOptions, io::prelude::*, io::BufRead};

pub struct Blacklist {
    addrs: Vec<String>,
    persistent: bool,
}

const BLACKLIST_FILE: &str = ".blacklist";

impl Blacklist {
    pub fn new(persistent: bool) -> Self {
        if persistent {
            if let Ok(file) = std::fs::File::open(BLACKLIST_FILE) {
                let lines = std::io::BufReader::new(file).lines();
                if let Ok(addrs) = lines
                    .into_iter()
                    .map(|l| l)
                    .collect::<Result<Vec<String>, _>>()
                {
                    return Blacklist {
                        addrs,
                        persistent: true,
                    };
                }
            }
            return Blacklist {
                addrs: Vec::default(),
                persistent: true,
            };
        }

        Blacklist {
            addrs: Vec::default(),
            persistent: false,
        }
    }

    pub fn add_addr<S: Into<String>>(&mut self, addr: S) {
        let addr = addr.into();
        tracing::info!("Save {} to blacklist", addr);

        if self.persistent {
            match OpenOptions::new()
                .create(true)
                .append(true)
                .open(BLACKLIST_FILE)
            {
                Ok(mut file) => {
                    if let Err(e) = writeln!(file, "{}", addr) {
                        tracing::warn!("Open file .blacklist failed, due to {:?}", e)
                    }
                }
                Err(e) => tracing::warn!("Open file .blacklist failed, due to {:?}", e),
            }
        }

        self.addrs.push(addr)
    }

    pub fn contains_addr(&self, addr: &str) -> bool {
        self.addrs.iter().find(|a| *a == addr).is_some()
    }
}
