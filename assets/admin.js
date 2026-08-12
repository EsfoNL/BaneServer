const delay = ms => new Promise(resolve => setTimeout(resolve, ms));

const CHUNKSIZE = 262144; // 256Kib


/**
* @param {Blob} file - the blob to chunk
* @returns {[Blob]} - the blob chunks
*/
function chunk_blob(file) {
  let chunk_count = Math.ceil(file.size / CHUNKSIZE);
  let chunks = [];

  for (let i = 0; i < chunk_count; i++) {
    chunks.push(file.slice(i * CHUNKSIZE, Math.max((i + 1) * CHUNKSIZE), file.size))
  }

  return chunks;
}


/**
* @param {string} base - the base path
* @param {HTMLElement} progress - Progress div
* @param {File} file - file to upload
*/
async function upload_file(base, file, progress) {
  let chunks = chunk_blob(file);
  // console.log(chunks);
  let res = await fetch(base + "create/" + encodeURIComponent(file.name), {method: "POST"});

  const uuid = await res.text();
  let url = base + "chunk/" + encodeURIComponent(uuid);
  let bar = progress.querySelector('progress');
  for (let i = 0; i < chunks.length; i++) {
    await fetch(url, {method: "POST", headers: {"byte-offset": CHUNKSIZE * i}, body: chunks[i]});
    bar.value = (i + 1) / chunks.length * 100;
  }

  await fetch(base + "finish/" + encodeURIComponent(uuid), {method: "POST"});
}
const uploader = (button, input, progress, path) => (() => {
  if (input.files.length < 1) {
    console.log("no files selected");
    return;
  }
  let file = input.files[0];
  button.disabled = true; // prevent double transmissions
  progress.hidden = false;
  upload_file(path, file, progress)
    .finally(() => progress.hidden = true)
    .finally(() => setTimeout(() => button.disabled = false, 1000));
});

mission_button.onclick = uploader(mission_button, mission, mission_progress, "/admin/mission/");
modpack_button.onclick = uploader(modpack_button, modpack, modpack_progress, "/admin/modpack/")

const action = (act) => (() => fetch("/admin/action/" + act))
start.onclick = action("start");
stop.onclick = action("stop");
restart.onclick = action("restart");
