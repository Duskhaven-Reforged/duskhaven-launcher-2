// import { invoke } from "@tauri-apps/api/tauri";
import { appWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";

//let newsListEl: HTMLElement | null;
let animcontainer: HTMLElement | null;

let headers = new Headers();

headers.append("Content-Type", "application/json");
headers.append("Accept", "application/json");
headers.append("Access-Control-Allow-Credentials", "true");
headers.append("Access-Control-Allow-Origin", "*");
headers.append("Access-Control-Allow-Methods", "GET");

const url = import.meta.env.VITE_FILESERVER_URL;
//const key = import.meta.env.VITE_ACCESS_KEY;
//const playSound = new Audio("/audio/play.wav");

window.addEventListener("DOMContentLoaded", () => {
  //getNews();
  document
    .getElementById("titlebar-minimize")
    ?.addEventListener("click", () => appWindow.minimize());
  document
    .getElementById("titlebar-close")
    ?.addEventListener("click", () => appWindow.close());
  document
    .getElementById("animation-toggle")
    ?.addEventListener("click", toggleAnimation);
  document
    .getElementById("play-button")
    ?.addEventListener("click", downloadFiles);
});
function toggleAnimation(e: MouseEvent) {
  animcontainer = document.querySelector(".fogwrapper");
  const icon = e.target as HTMLElement;
  if (animcontainer != null) {
    if (animcontainer.style.display === "none") {
      animcontainer.style.display = "block";
      icon.setAttribute("icon", "mdi:clapperboard-off-outline");
    } else {
      animcontainer.style.display = "none";
      icon.setAttribute("icon", "mdi:clapperboard-open-outline");
    }
  }
}

// listen for progress updates
listen("DOWNLOAD_PROGRESS", (event) => {
  const progress: any = event.payload;
  const dlProgress: HTMLElement | null =
    document.querySelector(".download-progress");
  const dlText: HTMLElement | null = document.querySelector(
    ".download-container .text-center"
  );
  dlProgress!.style!.width = `${progress.percentage}%`;
  dlText!.innerHTML = `download progress: ${progress.percentage.toFixed(
    2
  )}% (${(progress.transfer_rate / 1000 / 1000).toFixed(2)} megabytes/sec)`;
});

// listen for download finished
listen("DOWNLOAD_FINISHED", () => {
  console.log("Download finished");
});

function downloadFiles() {
  //playAudio();
  console.log("STARTED");
  invoke("download_file", {
    url: `${url}patch-J.mpq`,
    destination: "C:/Games/patch-J.mpq",
  })
    .then(() => {
      console.log("Download started");
    })
    .catch((err) => {
      console.error("Failed to start download:", err);
    });
}

// async function getNews() {
//   const response = await fetch("https://duskhaven-news.glitch.me/changelog", { method: "GET", headers });
//   if (response.ok) {
//     const newsContainer = document.getElementById('newslist');
//     const data = await response.json();
//     data.forEach((newsItem: any) => {
//       let sanitized = newsItem["content"];
//       sanitized = sanitized.replace(/@(everyone|here)/g, "To all users");

//       // Replace **text** with "text"
//       sanitized = sanitized.replace(/\*{2}(.*?)\*{2}/g, "\$1");
//       sanitized = sanitized.replace(/_{2}(.*?)_{2}/g, "\$1");
//       sanitized = sanitized.replace(/<.*>/g, "");
//       sanitized = sanitized.replace(/\n/g, "<br />");
//       let channel = document.createElement("span");
//       channel.innerHTML = newsItem["channelName"] + ":<br />";
//       channel.style.fontWeight = "bold";
//       let content = document.createElement("span");
//       content.style.fontWeight = "normal";
//       content.innerHTML = sanitized + ":<br /><br />";
//       var newLI = document.createElement('li');

//       newLI.appendChild(channel).appendChild(content);

//       newsContainer?.appendChild(newLI);
//     });

//     //newsListEl = document.querySelector("#newsList");
//   } else {

//     console.error(`HTTP error: ${response.status}`);
//   }
// }

function onKonamiCode(cb: Function) {
  var input = "";
  var key = "38384040373937396665";
  document.addEventListener("keydown", function (e) {
    input += "" + e.keyCode;
    if (input === key) {
      return cb();
    }
    if (!key.indexOf(input)) return;
    input = "" + e.keyCode;
  });
}

onKonamiCode(function () {
  document.body.style.backgroundImage = "url('./src/assets/background.gif')";
});

// function playAudio() {

//   playSound.volume = 0.20;
//   playSound.play();
// }
