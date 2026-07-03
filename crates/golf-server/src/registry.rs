use crate::room::RoomCmd;
use rand::Rng;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::mpsc;

/// Unambiguous room-code alphabet (no 0/O, 1/I/L).
const ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
const CODE_LEN: usize = 4;

#[derive(Default)]
pub struct Registry {
    rooms: Mutex<HashMap<String, mpsc::Sender<RoomCmd>>>,
}

impl Registry {
    /// Reserve a fresh room code and register its command channel.
    pub fn create(&self, tx: mpsc::Sender<RoomCmd>) -> String {
        let mut rooms = self.rooms.lock().unwrap();
        loop {
            let code: String = {
                let mut rng = rand::rng();
                (0..CODE_LEN)
                    .map(|_| ALPHABET[rng.random_range(0..ALPHABET.len())] as char)
                    .collect()
            };
            if !rooms.contains_key(&code) {
                rooms.insert(code.clone(), tx.clone());
                return code;
            }
        }
    }

    pub fn get(&self, code: &str) -> Option<mpsc::Sender<RoomCmd>> {
        self.rooms.lock().unwrap().get(code).cloned()
    }

    pub fn remove(&self, code: &str) {
        self.rooms.lock().unwrap().remove(code);
    }
}
