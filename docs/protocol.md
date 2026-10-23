# WebSocket-Protokoll: Webchat

**Stand:** MVP (ein Raum, Anmeldung nur mit Namen)

---

## 1. Verbindung

| Was | Wert |
| --- | --- |
| Frontend aufrufen | `http://localhost:8080/` |
| WebSocket-Adresse | `ws://localhost:8080/ws` |
| Frontend-Dateien | Ordner `static/` im Repository, werden von Actix ausgeliefert |
| Format | Jede WebSocket-Nachricht ist **ein JSON-Objekt als Text** |

Da Actix auch die HTML-Seite ausliefert, laufen Frontend und WebSocket unter derselben Adresse. Es gibt keine CORS-Probleme.

---

## 2. Ablauf

1. Der Browser öffnet die WebSocket-Verbindung.
2. Der Browser schickt als **erste Nachricht** eine Anmeldung (`login`) mit dem Namen.
3. Der Server antwortet mit einer Begrüßung (`welcome`). Sie enthält die **eigene ID**.
4. Ab jetzt kann der Browser Chatnachrichten (`chat`) schicken.
5. Jede Chatnachricht verteilt der Server an **alle im Raum, auch an den Absender**. Das Frontend zeigt also nur an, was vom Server kommt, und fügt eigene Nachrichten nicht selbst in die Liste ein.
6. Wird die Verbindung geschlossen, entfernt der Server den Benutzer. Bei einer neuen Verbindung muss sich der Browser erneut anmelden und bekommt eine **neue ID**.

**Eigene Nachrichten erkennen:** `sender_id` einer Chatnachricht == `id` aus der Begrüßung.

**Fehler** beendet die Verbindung nicht. Der Server schickt eine Fehlernachricht (`error`), und die Verbindung bleibt offen.

**Heartbeat:** Der Server schickt regelmäßig WebSocket-Pings. Browser beantworten diese automatisch. Das Frontend muss dafür nichts tun.

---

## 3. Nachrichten: Browser → Server

### Anmeldung (`login`)

Muss die erste Nachricht nach dem Verbinden sein. Nur einmal pro Verbindung erlaubt.

| Feld | Typ | Beschreibung |
| --- | --- | --- |
| type | Text | immer `"login"` |
| name | Text | Anzeigename, nicht leer |

```json
{ "type": "login", "name": "Alice" }
```

> **Später:** Für Supabase kommt ein Feld `token` dazu. Der Server übernimmt den Namen dann aus dem geprüften Token.

### Chatnachricht (`chat`)

| Feld | Typ | Beschreibung |
| --- | --- | --- |
| type | Text | immer `"chat"` |
| text | Text | Inhalt, 1 bis 1000 Zeichen, nicht nur Leerzeichen |

```json
{ "type": "chat", "text": "Hallo 👋" }
```

> Im MVP gibt es nur den Raum `welcome`, deshalb hat die Nachricht kein Raum-Feld.

---

## 4. Nachrichten: Server → Browser

### Begrüßung (`welcome`)

Kommt einmal, direkt nach erfolgreicher Anmeldung.

| Feld | Typ | Beschreibung |
| --- | --- | --- |
| type | Text | immer `"welcome"` |
| id | Zahl | eigene ID, zum Erkennen eigener Nachrichten |
| name | Text | bestätigter Anzeigename |

```json
{ "type": "welcome", "id": 1, "name": "Alice" }
```

### Chatnachricht (`chat`)

Geht an alle im Raum, auch an den Absender.

| Feld | Typ | Beschreibung |
| --- | --- | --- |
| type | Text | immer `"chat"` |
| sender_id | Zahl | ID des Absenders |
| sender_name | Text | Name des Absenders |
| text | Text | Inhalt |
| room | Text | Raum, im MVP immer `"welcome"` |
| timestamp | Zahl | Millisekunden seit 1970 (UTC), vom Server gesetzt |

```json
{
  "type": "chat",
  "sender_id": 1,
  "sender_name": "Alice",
  "text": "Hallo 👋",
  "room": "welcome",
  "timestamp": 1761217200000
}
```

> `timestamp` kann in JavaScript direkt an `new Date(...)` übergeben werden.

### Fehler (`error`)

| Feld | Typ | Beschreibung |
| --- | --- | --- |
| type | Text | immer `"error"` |
| code | Text | maschinenlesbarer Fehlercode, siehe Abschnitt 5 |
| message | Text | lesbarer Text, kann direkt angezeigt werden |

```json
{ "type": "error", "code": "not_logged_in", "message": "Bitte zuerst anmelden." }
```

---

## 5. Fehlercodes

| Code | Bedeutung |
| --- | --- |
| `invalid_json` | Die Nachricht ist kein gültiges JSON oder hat nicht die erwarteten Felder. |
| `unknown_type` | Der Wert von `type` ist unbekannt. |
| `not_logged_in` | Eine Chatnachricht kam vor der Anmeldung. |
| `already_logged_in` | Eine zweite Anmeldung auf derselben Verbindung. |
| `invalid_name` | Der Name ist leer. |

**Wichtig für das Frontend:** Leere oder zu lange Chatnachricht­en verwirft der Server im MVP **ohne Fehlermeldung**. Das Frontend sollte deshalb selbst verhindern, dass leere Nachrichten abgeschickt werden, und das Eingabefeld auf 1000 Zeichen begrenzen.

---

- Feld `token` in `login` für die Supabase-Anmeldung
- Nachrichtenverlauf beim Verbinden
- Mehrere Räume: Raum-Feld in `chat` (Browser → Server), Nachrichten `join` und `leave`
- Eigene Fehlercodes für leere und zu lange Nachrichten
