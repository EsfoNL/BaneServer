import { uploader } from "./uploader.js";
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));

var ws_logs;
const blinkenlight_logs = document.getElementById("blinkenlight-logs");
const logs = document.getElementById("logs");
addEventListener("DOMContentLoaded", (event) => {
  console.log("loaded");
  let sleep = 1000;
  let onopen = () => {
    sleep = 1000;
    blinkenlight_logs.setAttribute("live", "live");
  }
  let onmsg = (m) => {
    logs.innerText += m.data;
    if (logs.textContent.length > 5000) {
      logs.innerText = logs.innerText.slice(-25000);
    }
  };
  let oncls = () => {
    blinkenlight_logs.removeAttribute("live");
    setTimeout(() => {
      ws_logs = new WebSocket("/admin/script/websocket/live-logs?no-initial=true");
      ws_logs.onmessage = onmsg;
      ws_logs.onclose = oncls;
      ws_logs.onopen = onopen;
      sleep = sleep * 2;

      console.log("reconnect", sleep);
    }, sleep)
  }
  ws_logs = new WebSocket("/admin/script/websocket/live-logs");
  ws_logs.onmessage = onmsg;
  ws_logs.onclose = oncls;
  ws_logs.onopen = onopen;

  logs.textContent = '';
});




mission_button.onclick = uploader(mission_button, mission, mission_progress, "/admin/mission/");
modpack_button.onclick = uploader(modpack_button, modpack, modpack_progress, "/admin/modpack/")

const action = (act) => (() => fetch("/admin/action/" + act, { method: "POST" }))
start.onclick = action("start");
document.getElementById("stop").onclick = action("stop");
restart.onclick = action("restart");


const players = document.getElementById("players");
const blinkenlight = document.getElementById("blinkenlight");
let livelogs_ws;
function setup() {
  livelogs_ws = new WebSocket("/admin/live-players");
  livelogs_ws.onclose = _ => {
    blinkenlight.removeAttribute("live");
    delay(1000).then(_ => setup())
  };
  livelogs_ws.onopen = _ => {
    blinkenlight.setAttribute("live", "live");
  };
  livelogs_ws.onmessage = (e) => {
    console.log(e);
    /**
    * @typedef {{name: string}[]} Players
    * @type {Players} data
    */
    let data = JSON.parse(e.data);
    players.innerHTML = "";
    for (let datum of data.map(e => e.name)) {
      players.innerText += datum;
      players.innerHTML += "<br>";
    }
  }

}

setup();
