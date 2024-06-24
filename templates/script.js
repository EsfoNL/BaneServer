var ws = null
function myClick() {
  if (!ws) {
    let file =
    document.getElementById('filepicker').files[0];
    console.log(file);
    var url = new URL('/api/filestream', window.location.origin);
    url.protocol = url.protocol = "ws";
    console.log(url)
    ws = new WebSocket(url);
    ws.onmessage = (m) => {
      ws.onmessage = handler;
      console.log(m);
      let e = document.getElementById('link');
      let link = `/streamrcv.html?v=${m.data}`;
      e.setAttribute('href', link);
      e.innerText = link;
    }
  }
}


function handler(m)  {
  
}
