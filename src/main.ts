// import { invoke } from "@tauri-apps/api/tauri";
import { appWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import { Patch, Progress } from "./patch";
import { open, ask, message } from '@tauri-apps/api/dialog';
import { appDataDir } from '@tauri-apps/api/path';


enum ButtonStates {
  PLAY = "Play",
  DOWNLOAD = "Download",
  UPDATE = "Update",
}

//let newsListEl: HTMLElement | null;
let animcontainer: HTMLElement | null;
let installDirectory = localStorage.getItem("installDirectory");
let patches: Array<Patch>;
let downloadArray: Array<Patch> = [];
const url = import.meta.env.VITE_FILESERVER_URL;
//const key = import.meta.env.VITE_ACCESS_KEY;
const playSound = new Audio("/audio/play.wav");
const playButton: HTMLButtonElement = document.getElementById("play-button") as HTMLButtonElement;
const statusText = playButton?.querySelector(".status-text");
const dlProgress: HTMLElement | null =
  document.querySelector(".download-progress");
const dlText: HTMLElement | null = document.querySelector(
  ".download-container .text-center"
);
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

  hasInstallDirectory();

  document.getElementById("titlebar-dir")?.addEventListener("click", setInstallDirectory)
  playButton?.addEventListener("click", handlePlayButton);
});

async function hasInstallDirectory() {
  const appdir = await appDataDir();
  if (!installDirectory) {
    const yes = await ask('no install directory set want to set one now?', 'Duskhaven');
    if (!yes) {
      installDirectory = appdir
      localStorage.setItem("installDirectory", appdir)
      await message(`Ok if you press download we will download this in the current directory ${appdir}`, 'Duskhaven');
      return;
    }
    else {
      setInstallDirectory();
    }
  }
  fetchPatches();
}

async function setInstallDirectory() {
  const appdir = await appDataDir();

  const selected = await open({
    directory: true,
    multiple: false,
    defaultPath: installDirectory || appdir,
  });

  if (Array.isArray(selected)) {
    installDirectory = selected[0];
    localStorage.setItem("installDirectory", installDirectory)
  } else if (selected === null) {
    // user cancelled the selection
  } else {
    installDirectory = selected;
    localStorage.setItem("installDirectory", installDirectory)
  }
  fetchPatches();
}

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

  dlProgress!.style!.width = `${progress.percentage}%`;
  dlText!.innerHTML = `download progress: ${downloadArray[progress.download_id].ObjectName} ${progress.percentage.toFixed(
    2
  )}% (${(progress.transfer_rate / 1000 / 1000).toFixed(2)} megabytes/sec)`;
});

// listen for download finished
listen("DOWNLOAD_FINISHED", (event: { payload: Progress }) => {
  if (event?.payload.download_id === downloadArray.length - 1) {
    setButtonState(ButtonStates.PLAY, false);
  }
});

async function startGame() {
  playAudio();
  invoke('open_app', { path: `${installDirectory}/dusk-wow.exe` })
    .then(message => console.log(message))
    .catch(error => console.error(error))
}
async function downloadFiles() {
  //playAudio();
  if (downloadArray) {
    const urls = downloadArray.map((patch) => {
      return `${url}${patch.ObjectName}`;
    })
    const destinations = downloadArray.map((patch) => {
      return `${installDirectory}/Data/${patch.ObjectName}`;
    })
    setButtonState(ButtonStates.UPDATE, true);
    await invoke("download_files", {
      urls: urls,
      destinations: destinations,
    })
      .then(() => {
        console.log("Download started");
      })
      .catch((err) => {
        setButtonState(ButtonStates.UPDATE, false);
        console.error("Failed to start download:", err);
      });
  }
  else {
    setButtonState(ButtonStates.PLAY, false);

  }
}

async function handlePlayButton() {
  const statusText = playButton.querySelector(".status-text");
  switch (statusText?.innerHTML) {
    case ButtonStates.PLAY: startGame();
      break;
    case ButtonStates.DOWNLOAD:
    case ButtonStates.UPDATE: downloadFiles();
      break;
  }
}

async function fetchPatches() {
  try {
    patches = await invoke('get_patches');
    dlText!.innerHTML = `getting patch info`;
  } catch (error) {
    dlText!.innerHTML = `there seems to be a problem getting the patches: ${error}`
    //console.error('Failed to fetch patches:', error);
  }
  downloadArray = [];
  for (const patch of patches) {
    try {
      const timeStamp: { secs_since_epoch: number } = await invoke('modified_time', { filePath: `${installDirectory}/Data/${patch.ObjectName}` });
      console.log((new Date(patch.LastChanged).getTime() / 1000) > timeStamp.secs_since_epoch);
      if (((new Date(patch.LastChanged).getTime() / 1000) > timeStamp.secs_since_epoch)) {
        await downloadArray.push(patch);
      }
    } catch (error) {
      console.log(error);
      await downloadArray.push(patch);
    }
  }
  if (downloadArray.length === 0) {
    dlText!.innerHTML = `ready to play`;
    setButtonState(ButtonStates.PLAY, false);
  }
  else if (downloadArray.length === patches.length) {
    dlText!.innerHTML = `press download to install the custom duskhaven patches`;
    setButtonState(ButtonStates.DOWNLOAD, false);
  }
  else {
    dlText!.innerHTML = `there is an update available, please press upda  te to get the new patches`;
    setButtonState(ButtonStates.UPDATE, false);
  }

}

function setButtonState(state: ButtonStates, disabled: boolean) {
  playButton.disabled = disabled;
  if (statusText)
    statusText.innerHTML = state;
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

function playAudio() {
  playSound.volume = 0.5;
  playSound.play();
}
