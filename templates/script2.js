function myClick(t) {
  v = document.getElementById('uuid');
    var url = new URL(`/api/filestream/${v.value}`, window.location.origin);
    url.protocol = url.protocol = "ws";
  let ws = new WebSocket(url);
}

var data = [];

function handler(m) {
  
}
