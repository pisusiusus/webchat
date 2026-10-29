const ws = new WebSocket(`ws://${location.host}/ws`);
const messageList = document.getElementById("messages");
const messageInput = document.getElementById("message-input");
const messageForm = document.getElementById("message-send-form");
const loginForm = document.getElementById("login-form");
const usernameInput = document.getElementById("username-input");
const roomsList = document.getElementById("rooms-list");
const createRoomForm = document.getElementById("create-room-form");
const roomInput = document.getElementById("room-input");
const activeRoomTitle = document.getElementById("active-room");
const errorMessage = document.getElementById("error-message");

let activeRoom = "welcome";

let loggedIn = false;
let rooms = [];
const joinedRooms = new Set();

function sendMessage(message){
    if(ws.readyState !== WebSocket.OPEN){
        errorMessage.textContent = "No connection to the server. Please reload the page.";
        return false;
    }

    errorMessage.textContent = "";
    ws.send(JSON.stringify(message));

    return true;
}

function updateChat(){
    const canChat = loggedIn && joinedRooms.has(activeRoom);

    messageInput.disabled = !canChat;
    messageForm.querySelector("button").disabled = !canChat;

    roomInput.disabled = !loggedIn;
    createRoomForm.querySelector("button").disabled = !loggedIn;
    activeRoomTitle.textContent = loggedIn ? (activeRoom || "Join a room to chat") : "Please log in";

    document.getElementById("rooms").hidden = !loggedIn;
    document.getElementById("chat").hidden = !loggedIn;

    loginForm.style.display = loggedIn ? "none" : "";
}

function switchRoom(room){
    if(!loggedIn || !joinedRooms.has(room)){
        return;
    }

    activeRoom = room;
    console.log("Aktiver Raum:", activeRoom);
    messageList.replaceChildren();
    updateChat();
    renderRooms();

    const message = {
        type: "history",
        room: activeRoom
    }

    sendMessage(message);
}

function renderRooms(){
    roomsList.replaceChildren();

    for (const room of rooms) {
        const newRoom = document.createElement("li");
        const roomButton = document.createElement("button");
        const isJoined = joinedRooms.has(room);

        roomButton.type = "button";
        roomButton.textContent = room + (isJoined ? " (joined)" : " (join)") + (room === activeRoom ? " - active" : "");
        roomButton.disabled = !loggedIn;

        roomButton.addEventListener("click", () => {
            if(isJoined){
                switchRoom(room);
            } else {
                sendMessage({type: "join_room", room: room});
            }
        })

        newRoom.appendChild(roomButton);

        if(isJoined){
            const leaveButton = document.createElement("button");
            leaveButton.type = "button";
            leaveButton.textContent = "Leave";
            leaveButton.disabled = !loggedIn;
            leaveButton.addEventListener("click", () => {
                sendMessage({type: "leave_room", room: room});
            })
            newRoom.appendChild(leaveButton);
        }

        roomsList.appendChild(newRoom);
    }
}

createRoomForm.addEventListener("submit", (event) => {
    event.preventDefault();

    const room = roomInput.value.trim();
    if(!loggedIn || !room){
        return;
    }

    sendMessage({type: "create_room", room: room});
});

setInterval(() => {
    if(loggedIn && ws.readyState === WebSocket.OPEN){
        ws.send(JSON.stringify({type: "get_rooms"}));
    }
}, 3000);

loginForm.addEventListener("submit", (event) => {
    event.preventDefault();

    console.log("Login-Formular abgeschickt");
    console.log("WebSocket readyState:", ws.readyState);

    const username = usernameInput.value.trim();
    if(loggedIn || !username){
        return;
    }

    const message = {
        type: "login",
        name: username
    };

    sendMessage(message);
});

messageForm.addEventListener("submit", (event) => {
    event.preventDefault();

    console.log("Message abgeschickt");

    const messageToSend = messageInput.value.trim();
    if(!loggedIn || !joinedRooms.has(activeRoom) || !messageToSend){
        return;
    }

    const message = {
        type: "chat",
        text: messageToSend,
        room: activeRoom
    };

    if(sendMessage(message)){
        messageInput.value = "";
    }
})

ws.addEventListener("message", (event) => {
    const response = JSON.parse(event.data);

    if(response.type == "welcome"){
        console.log("Client: Login erfolgreich");

        loggedIn = true;
        joinedRooms.add("welcome");
        usernameInput.disabled = true;
        loginForm.querySelector("button").disabled = true;

        switchRoom("welcome");
    }

    if(response.type == "chat" && response.room === activeRoom && joinedRooms.has(activeRoom)){
        const message = document.createElement("li");
        message.textContent = `${response.sender_name}: ${response.text}`;
        messageList.appendChild(message);
    }

    if (response.type == "room_list"){
        rooms = response.rooms.sort();
        renderRooms();
    }

    if(response.type == "room_joined"){
        joinedRooms.add(response.room);
        if(!rooms.includes(response.room)){
            rooms.push(response.room);
            rooms.sort();
        }
        switchRoom(response.room);
    }

    if(response.type == "room_left"){
        joinedRooms.delete(response.room);

        if(activeRoom === response.room){
            activeRoom = null;
            messageList.replaceChildren();
            const nextRoom = joinedRooms.values().next().value;
            if(nextRoom){
                switchRoom(nextRoom);
            }
        }

        updateChat();
        renderRooms();
    }

    if(response.type === "history" && response.room === activeRoom && joinedRooms.has(activeRoom)){
        messageList.replaceChildren();

        for (const msg of response.messages){
            const message = document.createElement("li");
            message.textContent = `${msg.sender_name}: ${msg.text}`;
            messageList.appendChild(message);
        }
    }

    if(response.type == "error"){
        console.log("Fehler:", response.message);
        errorMessage.textContent = response.message;
    }
});




//debug Logs
ws.addEventListener("open", () => {
    console.log("WebSocket geöffnet");
});

ws.addEventListener("close", (event) => {
    loggedIn = false;
    joinedRooms.clear();
    updateChat();
    renderRooms();
    errorMessage.textContent = "Connection closed. Please reload the page.";

    console.log("WebSocket geschlossen:", {
        code: event.code,
        reason: event.reason,
        wasClean: event.wasClean
    });
});

ws.addEventListener("error", (event) => {
    console.error("WebSocket-Fehler:", event);
    errorMessage.textContent = "Connection failed. Please reload the page.";
});

