use std::collections::HashSet;

use serde::Serialize;

// Struct für die Messages, die im Verlauf gespeichert werden.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ChatMessage {
    pub sender_id: u64,
    pub sender_name: String,
    pub text: String,
    pub timestamp: u64,
}

/// Ein Chatraum. Speichert, welche Clients Mitglied sind und den Nachrichtenverlauf.
/// Namen und Adressen der Clients verwaltet der `ChatServer`.
pub struct ChatRoom {
    pub clients: HashSet<u64>,
    pub messages: Vec<ChatMessage>,
}

impl ChatRoom {
    /// Erstellt einen leeren Raum.
    pub fn new() -> Self {
        Self {
            clients: HashSet::new(),
            messages: Vec::new(),
        }
    }

    /// Fügt einen Client hinzu.
    /// Gibt `false` zurück, wenn er schon Mitglied war.
    pub fn join(&mut self, client_id: u64) -> bool {
        self.clients.insert(client_id)
    }

    /// Entfernt einen Client.
    /// Gibt `false` zurück. wenn er kein Mitglied war.
    pub fn leave(&mut self, client_id: u64) -> bool {
        self.clients.remove(&client_id)
    }

    /// Gibt die IDs aller Mitglieder zurück.
    pub fn get_members(&self) -> &HashSet<u64> {
        &self.clients
    }

    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }
}
