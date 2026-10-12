const ws = new WebSocket(`ws://${location.host}/ws`);
const list = document.getElementById("messages");
const input = document.getElementById("input");

ws.onmessage = (e) => {
    const li = document.createElement("li");
    li.textContent = e.data;
    list.appendChild(li);
};

document.getElementById("send").onclick = () => {
    if (input.value) {
        ws.send(input.value);
        input.value = "";
    }
};