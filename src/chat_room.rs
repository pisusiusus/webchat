
use std::collections::{HashMap,HashSet};

pub struct ChatRoom{
    pub clients: HashSet<u64>,  
}

impl ChatRoom {
    pub fn new() -> Self {
        Self {
            clients: HashSet::new(),
        }
    }

    pub fn join(&mut self, client_id: u64) -> bool {
        self.clients.insert(client_id)
    }

    pub fn leave(&mut self, client_id: u64) {
        self.clients.remove(&client_id);
    }

    pub fn get_members(&self) -> &HashSet<u64> {
        &self.clients 
    }

}

