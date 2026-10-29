const scheme = location.protocol === "https:" ? "wss" : "ws";
const ws = new WebSocket(`${scheme}://${location.host}/ws`);

const status = document.getElementById("status");
const loginForm = document.getElementById("login-form");
const usernameInput = document.getElementById("username-input");
const createRoomForm = document.getElementById("create-room-form");
const createRoomButton = document.getElementById("create-room-button");
const roomNameInput = document.getElementById("room-name-input");
const roomsList = document.getElementById("rooms-list");
const roomTitle = document.getElementById("active-room-title");
const messageList = document.getElementById("messages");
const messageInput = document.getElementById("message-input");
const messageForm = document.getElementById("message-send-form");
const sendButton = document.getElementById("send-button");

let loggedIn = false;
let activeRoom = null;
let rooms = [];
let joinedRooms = new Set();
let loadingHistory = false;
let roomActionPending = false;
let bufferedMessages = [];

function send(message) {
    if (ws.readyState !== WebSocket.OPEN) {
        status.textContent = "WebSocket is not connected.";
        return false;
    }
    ws.send(JSON.stringify(message));
    return true;
}

function updateControls() {
    const busy = loadingHistory || roomActionPending;
    const canChat = loggedIn && activeRoom !== null &&
        joinedRooms.has(activeRoom) && !busy;
    messageInput.disabled = !canChat;
    sendButton.disabled = !canChat;
    createRoomButton.disabled = !loggedIn || busy;
    roomNameInput.disabled = !loggedIn || busy;
    roomTitle.textContent = activeRoom ? `Room: ${activeRoom}` : "No room selected";
}

function appendMessage(message) {
    const item = document.createElement("li");
    item.textContent = `${message.sender_name}: ${message.text}`;
    messageList.appendChild(item);
}

function selectRoom(room) {
    if (loadingHistory || roomActionPending || !joinedRooms.has(room)) return;
    activeRoom = room;
    messageList.replaceChildren();
    bufferedMessages = [];
    loadingHistory = true;
    if (!send({ type: "history", room })) loadingHistory = false;
    updateControls();
    renderRooms();
}

function requestRoomAction(type, room) {
    if (!loggedIn || loadingHistory || roomActionPending) return;
    roomActionPending = true;
    if (!send({ type, room })) roomActionPending = false;
    updateControls();
    renderRooms();
}

function renderRooms() {
    roomsList.replaceChildren();
    const busy = loadingHistory || roomActionPending;
    for (const room of rooms) {
        const item = document.createElement("li");
        const joined = joinedRooms.has(room);
        const label = document.createElement("span");
        label.textContent = `${room}${joined ? " (joined)" : ""}` +
            `${room === activeRoom ? " — active" : ""} `;
        item.appendChild(label);

        const primary = document.createElement("button");
        primary.type = "button";
        primary.textContent = joined ? "Open" : "Join";
        primary.disabled = !loggedIn || busy;
        primary.addEventListener("click", () => {
            if (joined) selectRoom(room);
            else requestRoomAction("join_room", room);
        });
        item.appendChild(primary);

        if (joined) {
            const leave = document.createElement("button");
            leave.type = "button";
            leave.textContent = "Leave";
            leave.disabled = !loggedIn || busy;
            leave.addEventListener("click", () => requestRoomAction("leave_room", room));
            item.appendChild(leave);
        }
        roomsList.appendChild(item);
    }
}

loginForm.addEventListener("submit", (event) => {
    event.preventDefault();
    const name = usernameInput.value.trim();
    if (name) send({ type: "login", name });
});

createRoomForm.addEventListener("submit", (event) => {
    event.preventDefault();
    const room = roomNameInput.value.trim();
    if (!/^[A-Za-z0-9_-]{1,40}$/.test(room)) {
        status.textContent = "Use 1-40 letters, numbers, '-' or '_' for room names.";
        return;
    }
    requestRoomAction("create_room", room);
});

messageForm.addEventListener("submit", (event) => {
    event.preventDefault();
    if (!loggedIn || !activeRoom || !joinedRooms.has(activeRoom) ||
        loadingHistory || roomActionPending) return;
    const text = messageInput.value.trim();
    if (text && send({ type: "chat", text, room: activeRoom })) {
        messageInput.value = "";
    }
});

ws.addEventListener("message", (event) => {
    let response;
    try {
        response = JSON.parse(event.data);
    } catch {
        status.textContent = "Server sent invalid JSON.";
        return;
    }

    switch (response.type) {
        case "welcome": {
            loggedIn = true;
            loginForm.hidden = true;
            status.textContent = `Logged in as ${response.name}.`;
            updateControls();
            break;
        }
        case "room_list": {
            rooms = response.rooms;
            joinedRooms = new Set(response.joined_rooms);
            if (activeRoom && !joinedRooms.has(activeRoom)) {
                activeRoom = null;
                loadingHistory = false;
                bufferedMessages = [];
                messageList.replaceChildren();
            }
            if (!activeRoom && joinedRooms.size > 0 && !roomActionPending) {
                const next = joinedRooms.has("welcome") ? "welcome" :
                    [...joinedRooms].sort()[0];
                selectRoom(next);
            } else {
                renderRooms();
                updateControls();
            }
            break;
        }
        case "room_joined": {
            roomActionPending = false;
            joinedRooms.add(response.room);
            if (!rooms.includes(response.room)) {
                rooms.push(response.room);
                rooms.sort();
            }
            roomNameInput.value = "";
            status.textContent = `Joined ${response.room}.`;
            selectRoom(response.room);
            break;
        }
        case "room_left": {
            roomActionPending = false;
            joinedRooms.delete(response.room);
            if (activeRoom === response.room) {
                activeRoom = null;
                loadingHistory = false;
                bufferedMessages = [];
                messageList.replaceChildren();
                const next = [...joinedRooms].sort()[0];
                if (next) selectRoom(next);
            }
            status.textContent = `Left ${response.room}.`;
            renderRooms();
            updateControls();
            break;
        }
        case "chat": {
            if (response.room !== activeRoom) break;
            if (loadingHistory) bufferedMessages.push(response);
            else appendMessage(response);
            break;
        }
        case "history": {
            if (response.room !== activeRoom) break;
            messageList.replaceChildren();
            for (const message of response.messages) appendMessage(message);

            const key = (message) => JSON.stringify([
                message.sender_id, message.sender_name, message.text, message.timestamp,
            ]);
            const counts = new Map();
            for (const message of response.messages) {
                const k = key(message);
                counts.set(k, (counts.get(k) ?? 0) + 1);
            }
            for (const message of bufferedMessages) {
                const k = key(message);
                const count = counts.get(k) ?? 0;
                if (count > 0) counts.set(k, count - 1);
                else appendMessage(message);
            }
            bufferedMessages = [];
            loadingHistory = false;
            renderRooms();
            updateControls();
            break;
        }
        case "error": {
            status.textContent = response.message;
            console.error(response.code, response.message);
            roomActionPending = false;
            loadingHistory = false;
            for (const message of bufferedMessages) appendMessage(message);
            bufferedMessages = [];
            renderRooms();
            updateControls();
            break;
        }
    }
});

ws.addEventListener("open", () => {
    status.textContent = "Connected. Please log in.";
});
ws.addEventListener("close", () => {
    loggedIn = false;
    loadingHistory = false;
    roomActionPending = false;
    status.textContent = "Disconnected. Reload the page to reconnect.";
    renderRooms();
    updateControls();
});
ws.addEventListener("error", () => {
    status.textContent = "WebSocket connection error.";
});

setInterval(() => {
    if (loggedIn && !loadingHistory && !roomActionPending &&
        ws.readyState === WebSocket.OPEN) {
        send({ type: "list_rooms" });
    }
}, 3000);

updateControls();
