# WebSocket-Protokoll

So kommunizieren Browser und Server miteinander.

## Verbindung

- Frontend: `http://localhost:8080/`
- WebSocket: `ws://localhost:8080/ws`
- Frontend-Dateien gehören in den Ordner `static/`, Actix liefert sie aus.
- Jede Nachricht ist **ein JSON-Objekt als Text**.

## Ablauf

1. Verbindung öffnen.
2. Als erste Nachricht ein `login` mit Namen schicken.
3. Der Server antwortet mit `welcome`. Darin steht die **eigene ID**.
4. Danach können `chat`-Nachrichten geschickt werden.
5. Jede Chatnachricht geht an alle im Raum, **auch an den Absender selbst**. Das Frontend zeigt also nur an, was vom Server kommt, und fügt eigene Nachrichten nicht selbst in die Liste ein.

Gut zu wissen:

- **Eigene Nachrichten erkennen:** `sender_id` == `id` aus `welcome`.
- **Fehler** schließen die Verbindung nicht. Es kommt eine `error`-Nachricht, danach geht es normal weiter.
- **Neue Verbindung = neue ID.** Nach einem Neuladen der Seite ist ein neues `login` nötig.
- **Heartbeat:** Der Server schickt regelmäßig Pings. Browser beantworten sie automatisch, im Frontend ist dafür nichts zu tun.

---

## Browser → Server

### `login`

Einmal pro Verbindung, als erste Nachricht.

| Feld | Typ | Bedeutung |
| --- | --- | --- |
| type | Text | `"login"` |
| name | Text | Anzeigename, darf nicht leer sein |

```json
{ "type": "login", "name": "Alice" }
```

### `chat`

| Feld | Typ | Bedeutung |
| --- | --- | --- |
| type | Text | `"chat"` |
| text | Text | 1 bis 1000 Zeichen, nicht nur Leerzeichen |

```json
{ "type": "chat", "text": "Hallo 👋" }
```

Es gibt nur einen Raum (`welcome`), deshalb kein Raum-Feld.

---

## Server → Browser

### `welcome`

Kommt einmal, direkt nach erfolgreichem Login.

| Feld | Typ | Bedeutung |
| --- | --- | --- |
| type | Text | `"welcome"` |
| id | Zahl | eigene ID, zum Erkennen eigener Nachrichten |
| name | Text | bestätigter Anzeigename |

```json
{ "type": "welcome", "id": 1, "name": "Alice" }
```

### `chat`

| Feld | Typ | Bedeutung |
| --- | --- | --- |
| type | Text | `"chat"` |
| sender_id | Zahl | ID des Absenders |
| sender_name | Text | Name des Absenders |
| text | Text | Inhalt |
| room | Text | immer `"welcome"` |
| timestamp | Zahl | Millisekunden seit 1970 (UTC), funktioniert direkt mit `new Date(...)` |

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

### `error`

| Feld | Typ | Bedeutung |
| --- | --- | --- |
| type | Text | `"error"` |
| code | Text | Fehlercode, siehe unten |
| message | Text | lesbarer Text, kann direkt angezeigt werden |

```json
{ "type": "error", "code": "not_logged_in", "message": "Bitte zuerst anmelden." }
```

---

## Fehlercodes

| Code | Wann |
| --- | --- |
| `invalid_json` | Kein gültiges JSON, unbekannter `type` oder falsche Felder |
| `not_logged_in` | `chat` vor dem `login` |
| `already_logged_in` | zweites `login` auf derselben Verbindung |
| `invalid_name` | leerer Name beim `login` |

**Wichtig:** Leere oder zu lange Chatnachrichten verwirft der Server **ohne Fehlermeldung**. Am besten verhindert das Frontend, dass leere Nachrichten abgeschickt werden, und begrenzt das Eingabefeld auf 1000 Zeichen.