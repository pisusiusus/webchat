const ws = new WebSocket(`ws://${location.host}/ws`);
const messageList = document.getElementById("messages");
const messageInput = document.getElementById("message-input");
const messageForm = document.getElementById("message-send-form");
const loginForm = document.getElementById("login-form");
const usernameInput = document.getElementById("username-input");
const roomsList = document.getElementById("rooms-list");

let activeRoom = "welcome";
let isLoggedIn = false;

loginForm.style.display = "block";
messageForm.style.display = "none";

loginForm.addEventListener("submit", (event) => {
    event.preventDefault();

    console.log("Login-Formular abgeschickt");
    console.log("WebSocket readyState:", ws.readyState);

    const username = usernameInput.value.trim();

    const message = {
        type: "login",
        name: username
    };

    ws.send(JSON.stringify(message));
});

messageForm.addEventListener("submit", (event) => {
    event.preventDefault();

    console.log("Message abgeschickt");

    const messageToSend = messageInput.value.trim();

    const message = {
        type: "chat",
        text: messageToSend,
        room: activeRoom
    };

    ws.send(JSON.stringify(message));
})

ws.addEventListener("message", (event) => {
    const response = JSON.parse(event.data);

    if(response.type == "welcome"){
        console.log("Client: Login erfolgreich");
        isLoggedIn = true;

        loginForm.style.display = "none";
        messageForm.style.display = "block";
    }

    if(response.type == "chat"){
        const message = document.createElement("li");
        message.textContent = `${response.sender_name}: ${response.text}`;
        messageList.appendChild(message);
    }

    if (response.type == "room_list"){
        for (var room of response.rooms) {
            const newRoom = document.createElement("li");
            newRoom.textContent = room;

            newRoom.addEventListener("click", () => {
                activeRoom = room;
                console.log("Aktiver Raum:", activeRoom);

                const message = {
                    type: "history",
                    room: activeRoom
                }

                ws.send(JSON.stringify(message));
            })

            roomsList.appendChild(newRoom);
        }
    }

    if(response.type === "history"){
        messageList.replaceChildren();

        for (const msg of response.messages){
            const message = document.createElement("li");
            message.textContent = `${msg.sender_name}: ${msg.text}`;
            messageList.appendChild(message);
        }
    }

    if(response.type == "error"){
        console.log("Fehler:");
    }
});

//debug Logs
ws.addEventListener("open", () => {
    console.log("WebSocket geöffnet");
});

ws.addEventListener("close", (event) => {
    console.log("WebSocket geschlossen:", {
        code: event.code,
        reason: event.reason,
        wasClean: event.wasClean
    });
});

ws.addEventListener("error", (event) => {
    console.error("WebSocket-Fehler:", event);
});

