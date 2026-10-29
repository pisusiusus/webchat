const $ = (id) => document.getElementById(id);
const ws = new WebSocket(`${location.protocol === "https:" ? "wss" : "ws"}://${location.host}/ws`);
const loginDialog = $("login-dialog");
const createDialog = $("create-dialog");
const messageInput = $("message-input");
const messageList = $("messages");
const scrollArea = $("message-scroll");
let loggedIn = false;
let currentUserId = null;
let currentName = "";
let activeRoom = null;
let rooms = [];
let joinedRooms = new Set();
let loadingHistory = false;
let roomActionPending = false;
let loginPending = false;
let bufferedMessages = [];
let lastMessageDay = null;
let toastTimer;

function showToast(text, error = false) {
    clearTimeout(toastTimer);
    $("toast").textContent = text;
    $("toast").classList.toggle("error", error);
    $("toast").hidden = false;
    toastTimer = setTimeout(() => { $("toast").hidden = true; }, 4500);
}
function send(message) {
    if (ws.readyState !== WebSocket.OPEN) {
        showToast("Disconnected. Reload the page to reconnect.", true);
        return false;
    }
    ws.send(JSON.stringify(message));
    return true;
}
function setConnection(text, state) {
    const label = $("connection-label");
    label.replaceChildren();
    const dot = document.createElement("i");
    dot.className = "connection-dot";
    label.append(dot, document.createTextNode(text));
    document.body.classList.toggle("is-connected", state === "connected");
    document.body.classList.toggle("is-disconnected", state === "disconnected");
}
function initials(name) {
    return Array.from(name.trim()).slice(0, 2).join("").toUpperCase() || "?";
}
function avatarColor(name) {
    const colors = ["#5865f2", "#9858bd", "#258d80", "#b15a69", "#a47135", "#3979b5"];
    let hash = 0;
    for (const char of name) hash = (hash * 31 + char.codePointAt(0)) >>> 0;
    return colors[hash % colors.length];
}
function setSidebar(open) {
    document.body.classList.toggle("sidebar-open", open);
    $("sidebar-backdrop").hidden = !open;
    $("sidebar-toggle").setAttribute("aria-expanded", String(open));
    if (!open) $("sidebar-toggle").focus({ preventScroll: true });
}
function updateControls() {
    const connected = ws.readyState === WebSocket.OPEN;
    const busy = loadingHistory || roomActionPending;
    const canChat = connected && loggedIn && activeRoom !== null &&
        joinedRooms.has(activeRoom) && !busy;
    messageInput.disabled = !canChat;
    $("send-button").disabled = !canChat;
    $("character-count").textContent = `${messageInput.value.length}/200`;
    $("character-count").classList.toggle("near-limit", messageInput.value.length >= 180);
    messageInput.placeholder = activeRoom ? `Message #${activeRoom}` : "Select a channel to start chatting";
    for (const id of ["rail-create", "open-create", "create-channel-link", "create-room-button"]) {
        $(id).disabled = !connected || !loggedIn || busy;
    }
    $("room-name-input").disabled = roomActionPending;
    $("leave-channel").hidden = !activeRoom || !joinedRooms.has(activeRoom);
    $("leave-channel").disabled = !connected || !loggedIn || busy;
    $("history-state").hidden = !loadingHistory;
    $("channel-title").textContent = activeRoom || "WebChat";
    $("welcome-title").textContent = activeRoom ? `Welcome to #${activeRoom}!` : "Make yourself at home.";
    $("welcome-description").textContent = activeRoom
        ? `This is the beginning of the #${activeRoom} channel.`
        : "Join a channel on the left, or create a space of your own.";
    document.title = activeRoom ? `#${activeRoom} · WebChat` : "WebChat";
    $("login-button").disabled = !connected || loginPending;
    $("login-button").textContent = !connected ? "Connecting…" : loginPending ? "Joining…" : "Let's go";
}
function clearMessages() {
    messageList.replaceChildren();
    lastMessageDay = null;
}
function appendMessage(message) {
    const date = new Date(message.timestamp);
    const validDate = !Number.isNaN(date.getTime());
    const day = validDate ? date.toLocaleDateString() : "";
    if (day && day !== lastMessageDay) {
        const divider = document.createElement("li");
        divider.className = "day-divider";
        divider.textContent = date.toLocaleDateString(undefined, { month: "long", day: "numeric", year: "numeric" });
        messageList.appendChild(divider);
        lastMessageDay = day;
    }
    const item = document.createElement("li");
    item.className = "message";
    const avatar = document.createElement("div");
    avatar.className = "avatar";
    avatar.textContent = initials(message.sender_name);
    avatar.style.setProperty("--avatar-color", avatarColor(message.sender_name));
    avatar.setAttribute("aria-hidden", "true");
    const body = document.createElement("div");
    body.className = "message-body";
    const meta = document.createElement("div");
    meta.className = "message-meta";
    const author = document.createElement("span");
    author.className = "message-author";
    author.textContent = message.sender_name;
    meta.appendChild(author);
    if (message.sender_id === currentUserId) {
        const tag = document.createElement("span");
        tag.className = "self-tag";
        tag.textContent = "YOU";
        meta.appendChild(tag);
    }
    if (validDate) {
        const time = document.createElement("time");
        time.className = "message-time";
        time.dateTime = date.toISOString();
        time.textContent = date.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
        time.title = date.toLocaleString();
        meta.appendChild(time);
    }
    const text = document.createElement("p");
    text.className = "message-text";
    text.textContent = message.text;
    body.append(meta, text);
    item.append(avatar, body);
    messageList.appendChild(item);
}
function nearBottom() {
    return scrollArea.scrollHeight - scrollArea.scrollTop - scrollArea.clientHeight < 120;
}
function scrollToBottom() { scrollArea.scrollTop = scrollArea.scrollHeight; }
function selectRoom(room) {
    if (loadingHistory || roomActionPending || !joinedRooms.has(room)) return;
    activeRoom = room;
    clearMessages();
    bufferedMessages = [];
    loadingHistory = true;
    if (!send({ type: "history", room })) loadingHistory = false;
    if (document.body.classList.contains("sidebar-open")) setSidebar(false);
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
function makeRoomRow(room, joined) {
    const item = document.createElement("li");
    item.className = `channel-row${room === activeRoom ? " active" : ""}`;
    const open = document.createElement("button");
    open.type = "button";
    open.className = "channel-open";
    open.title = joined ? `Open #${room}` : `Join #${room}`;
    if (room === activeRoom) open.setAttribute("aria-current", "page");
    const hash = document.createElement("span");
    hash.className = "channel-hash";
    hash.textContent = "#";
    hash.setAttribute("aria-hidden", "true");
    const name = document.createElement("span");
    name.className = "channel-name";
    name.textContent = room;
    open.append(hash, name);
    const disabled = !loggedIn || loadingHistory || roomActionPending || ws.readyState !== WebSocket.OPEN;
    open.disabled = disabled;
    open.addEventListener("click", () => joined ? selectRoom(room) : requestRoomAction("join_room", room));
    item.appendChild(open);
    if (!joined) {
        const join = document.createElement("button");
        join.type = "button";
        join.className = "join-button";
        join.textContent = "Join";
        join.setAttribute("aria-label", `Join #${room}`);
        join.disabled = disabled;
        join.addEventListener("click", () => requestRoomAction("join_room", room));
        item.appendChild(join);
    }
    return item;
}
function renderRooms() {
    $("joined-rooms").replaceChildren();
    $("available-rooms").replaceChildren();
    let joinedCount = 0;
    let availableCount = 0;
    for (const room of rooms) {
        const joined = joinedRooms.has(room);
        $(joined ? "joined-rooms" : "available-rooms").appendChild(makeRoomRow(room, joined));
        if (joined) joinedCount++; else availableCount++;
    }
    $("joined-count").textContent = joinedCount;
    $("discover-count").textContent = availableCount;
    $("no-joined").hidden = joinedCount > 0;
    $("no-available").hidden = availableCount > 0;
}
function openCreate() {
    if (!loggedIn || loadingHistory || roomActionPending || ws.readyState !== WebSocket.OPEN) return;
    $("create-error").hidden = true;
    if (!createDialog.open) createDialog.showModal();
    $("room-name-input").focus();
}
for (const id of ["rail-create", "open-create", "create-channel-link"]) $(id).addEventListener("click", openCreate);
for (const id of ["close-create", "cancel-create"]) $(id).addEventListener("click", () => createDialog.close());
$("sidebar-toggle").addEventListener("click", () => setSidebar(!document.body.classList.contains("sidebar-open")));
$("sidebar-backdrop").addEventListener("click", () => setSidebar(false));
document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && document.body.classList.contains("sidebar-open") && !createDialog.open) setSidebar(false);
});
loginDialog.addEventListener("cancel", (event) => event.preventDefault());
$("login-form").addEventListener("submit", (event) => {
    event.preventDefault();
    const name = $("username-input").value.trim();
    if (!name || loginPending) return;
    $("login-error").hidden = true;
    loginPending = true;
    if (!send({ type: "login", name })) loginPending = false;
    updateControls();
});
$("create-room-form").addEventListener("submit", (event) => {
    event.preventDefault();
    const room = $("room-name-input").value.trim();
    if (!/^[A-Za-z0-9_-]{1,40}$/.test(room)) {
        $("create-error").textContent = "Use 1–40 letters, numbers, underscores or hyphens.";
        $("create-error").hidden = false;
        return;
    }
    $("create-error").hidden = true;
    requestRoomAction("create_room", room);
});
$("leave-channel").addEventListener("click", () => {
    if (activeRoom) requestRoomAction("leave_room", activeRoom);
});
messageInput.addEventListener("input", updateControls);
$("message-send-form").addEventListener("submit", (event) => {
    event.preventDefault();
    if (!loggedIn || !activeRoom || !joinedRooms.has(activeRoom) || loadingHistory || roomActionPending) return;
    const text = messageInput.value.trim();
    if (text && send({ type: "chat", text, room: activeRoom })) {
        messageInput.value = "";
        updateControls();
        messageInput.focus();
    }
});
ws.addEventListener("message", (event) => {
    let response;
    try { response = JSON.parse(event.data); }
    catch { showToast("The server sent an invalid response.", true); return; }
    switch (response.type) {
        case "welcome": {
            loggedIn = true;
            loginPending = false;
            currentUserId = response.id;
            currentName = response.name;
            $("current-user").textContent = currentName;
            $("user-avatar").textContent = initials(currentName);
            $("user-avatar").style.setProperty("--avatar-color", avatarColor(currentName));
            loginDialog.close();
            setConnection("Connected", "connected");
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
                clearMessages();
            }
            if (!activeRoom && joinedRooms.size && !roomActionPending) {
                selectRoom(joinedRooms.has("welcome") ? "welcome" : [...joinedRooms].sort()[0]);
            } else { renderRooms(); updateControls(); }
            break;
        }
        case "room_joined": {
            roomActionPending = false;
            joinedRooms.add(response.room);
            if (!rooms.includes(response.room)) { rooms.push(response.room); rooms.sort(); }
            if (createDialog.open) createDialog.close();
            $("room-name-input").value = "";
            selectRoom(response.room);
            showToast(`You're now in #${response.room}.`);
            break;
        }
        case "room_left": {
            roomActionPending = false;
            joinedRooms.delete(response.room);
            if (activeRoom === response.room) {
                activeRoom = null;
                loadingHistory = false;
                bufferedMessages = [];
                clearMessages();
                const next = [...joinedRooms].sort()[0];
                if (next) selectRoom(next);
            }
            renderRooms(); updateControls();
            showToast(`You left #${response.room}.`);
            break;
        }
        case "chat": {
            if (response.room !== activeRoom) break;
            if (loadingHistory) bufferedMessages.push(response);
            else {
                const shouldScroll = nearBottom() || response.sender_id === currentUserId;
                appendMessage(response);
                if (shouldScroll) scrollToBottom();
            }
            break;
        }
        case "history": {
            if (response.room !== activeRoom) break;
            clearMessages();
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
                const k = key(message), count = counts.get(k) ?? 0;
                if (count > 0) counts.set(k, count - 1); else appendMessage(message);
            }
            bufferedMessages = [];
            loadingHistory = false;
            renderRooms(); updateControls(); scrollToBottom();
            if (window.matchMedia("(min-width: 701px)").matches && !createDialog.open) messageInput.focus();
            break;
        }
        case "error": {
            loginPending = false;
            roomActionPending = false;
            loadingHistory = false;
            for (const message of bufferedMessages) appendMessage(message);
            bufferedMessages = [];
            if (loginDialog.open) {
                $("login-error").textContent = response.message;
                $("login-error").hidden = false;
            } else if (createDialog.open) {
                $("create-error").textContent = response.message;
                $("create-error").hidden = false;
            } else showToast(response.message, true);
            renderRooms(); updateControls();
            console.error(response.code, response.message);
            break;
        }
    }
});
ws.addEventListener("open", () => {
    setConnection("Connected", "connected");
    updateControls();
});
ws.addEventListener("close", () => {
    loggedIn = false;
    loginPending = false;
    loadingHistory = false;
    roomActionPending = false;
    bufferedMessages = [];
    setConnection("Disconnected", "disconnected");
    $("connection-banner").textContent = "Connection lost. Reload this page to reconnect.";
    $("connection-banner").hidden = false;
    if (loginDialog.open) {
        $("login-error").textContent = "Connection lost. Reload this page to reconnect.";
        $("login-error").hidden = false;
    }
    updateControls(); renderRooms();
    $("login-button").textContent = "Disconnected";
});
ws.addEventListener("error", () => {
    if (loginDialog.open) {
        $("login-error").textContent = "Can't reach the server. Check that cargo run is still running.";
        $("login-error").hidden = false;
    } else showToast("WebSocket connection error.", true);
});
setInterval(() => {
    if (loggedIn && !loadingHistory && !roomActionPending && ws.readyState === WebSocket.OPEN) send({ type: "list_rooms" });
}, 3000);
loginDialog.showModal();
updateControls();
renderRooms();
