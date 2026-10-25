use std::collections::HashSet;

/// Ein Chatraum. Speichert nur, welche Clients Mitglied sind.
/// Namen und Adressen der Clients verwaltet der `ChatServer`.
pub struct ChatRoom {
    pub clients: HashSet<u64>,
}

impl ChatRoom {
    /// Erstellt einen leeren Raum.
    pub fn new() -> Self {
        Self {
            clients: HashSet::new(),
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
}
