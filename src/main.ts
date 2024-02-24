import { appWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import { Patch, Progress } from "./patch";
import { open, ask, message } from "@tauri-apps/api/dialog";
import { appDataDir } from "@tauri-apps/api/path";
import { getClient, ResponseType } from "@tauri-apps/api/http";
// import i18n from "./i18n";

enum ButtonStates {
  PLAY = "Play",
  DOWNLOAD = "Download",
  UPDATE = "Update",
  VERIFY = "Verifying"
}
let animcontainer: HTMLElement | null;
let installDirectory = localStorage.getItem("installDirectory");
let autoPlay = localStorage.getItem("autoPlay") === "true";
let patches: Array<Patch>;
let downloadArray: Array<Patch> = [];
const url =
  process.env.VITE_FILESERVER_URL || import.meta.env.VITE_FILESERVER_URL;
const playSound = new Audio("/audio/play.wav");
const playButton: HTMLButtonElement = document.getElementById(
  "play-button"
) as HTMLButtonElement;
const autoPlayCheck: HTMLInputElement = document.getElementById(
  "autoplay"
) as HTMLInputElement;
const statusText = playButton?.querySelector(".status-text");
const dlProgress: HTMLElement | null =
  document.querySelector(".download-progress");
const directorySelector: HTMLButtonElement = document.getElementById(
  "titlebar-dir"
) as HTMLButtonElement;
const dlText: HTMLElement | null = document.querySelector(
  ".download-container .text-center"
);
window.addEventListener("DOMContentLoaded", () => {
  document
    .getElementById("titlebar-minimize")
    ?.addEventListener("click", () => appWindow.minimize());
  document
    .getElementById("titlebar-close")
    ?.addEventListener("click", () => appClose());
  document
    .getElementById("animation-toggle")
    ?.addEventListener("click", toggleAnimation);
  autoPlayCheck.checked = autoPlay;
  hasInstallDirectory();
  document.getElementById("autoplay")?.addEventListener("change", setAutoPlay);
  directorySelector?.addEventListener("click", setInstallDirectory);
  playButton?.addEventListener("click", handlePlayButton);
  getNews();
});

async function hasInstallDirectory() {
  const appdir = await appDataDir();
  if (!installDirectory) {
    const yes = await ask(
      "no install directory set want to set one now?",
      "Duskhaven"
    );
    if (!yes) {
      installDirectory = appdir;
      localStorage.setItem("installDirectory", appdir);
      await message(
        `Ok if you press download we will download this in the current directory ${appdir}`,
        "Duskhaven"
      );
      return;
    } else {
      setInstallDirectory();
    }
  }
  fetchPatches();
}


// async function setAccountDetails(username: string, password: string) {
//   invoke('update_account_info', {installDirectory, username, password})
//     .then(value => console.log("success", value))
//     .catch(error => console.log(error));
// }
async function setInstallDirectory() {
  if (directorySelector.disabled) {
    return;
  }
  const appdir = await appDataDir();

  const selected = await open({
    directory: true,
    multiple: false,
    defaultPath: installDirectory || appdir,
  });

  if (Array.isArray(selected)) {
    installDirectory = selected[0];
    localStorage.setItem("installDirectory", installDirectory);
  } else if (selected === null) {
    // user cancelled the selection
  } else {
    installDirectory = selected;
    localStorage.setItem("installDirectory", installDirectory);
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

function setAutoPlay(e: any) {
  localStorage.setItem("autoPlay", e.target.checked);
}

// listen for progress updates
listen("DOWNLOAD_PROGRESS", (event) => {
  const progress: any = event.payload;

  dlProgress!.style!.width = `${progress.percentage}%`;
  dlText!.innerHTML = `<div class="percent"> ${progress.percentage.toFixed(
    2
  )}%</div><div class="file">${downloadArray[progress.download_id].ObjectName
    } </div>  <div class="speed">(${(
      progress.transfer_rate /
      1000 /
      1000
    ).toFixed(2)} MB/sec)</div>`;
});

// listen for download finished
listen("DOWNLOAD_FINISHED", (event: { payload: Progress }) => {
  if (event?.payload.download_id === downloadArray.length - 1) {
    if (autoPlayCheck.checked) {
      startGame();
    }
    setButtonState(ButtonStates.PLAY, false);
  }
});

async function startGame() {
  playAudio();
  invoke("open_app", { path: `${installDirectory}/dusk-wow.exe` })
    .then((message) => console.log(message))
    .catch((error) =>
      message(`an error occurred!' ${error}`, { title: "Error (print screen this to get it solved)", type: "error" })
    );
}
async function downloadFiles() {
  if (downloadArray) {
    const urls = downloadArray.map((patch) => {
      return `${url}${patch.ObjectName}`;
    });
    const destinations = downloadArray.map((patch) => patch.filePath);
    setButtonState(ButtonStates.UPDATE, true);
    await invoke("download_files", {
      urls: urls,
      destinations: destinations,
    })
      .then(async (result) => {
        setButtonState(ButtonStates.VERIFY, true);
        for(const file of destinations) {
          dlText!.innerHTML = `<div class="percent"> Verifying downloaded files...</div>`;
          await getFileHash(file!, true);
          
        }
        setButtonState(ButtonStates.PLAY, false);
        console.log("file", result)
      })
      .catch((err) => {
        setButtonState(ButtonStates.UPDATE, false);
        message(`an error occurred!' ${err}`, {
          title: "Error",
          type: "error",
        });
      });
  } else {
    setButtonState(ButtonStates.PLAY, false);
  }
}

async function handlePlayButton() {
  const statusText = playButton.querySelector(".status-text");
  switch (statusText?.innerHTML) {
    case ButtonStates.PLAY:
      startGame();
      break;
    case ButtonStates.DOWNLOAD:
    case ButtonStates.UPDATE:
      downloadFiles();
      break;
  }
}

async function getFileHash(fileLocation: string, force= false ) {
  const fileName = fileLocation.split("/").pop()!;

  if (localStorage.getItem(fileName) && !force) {
    return localStorage.getItem(fileName)
  } else {
    return invoke("sha256_digest", { fileLocation })
      .then((result: unknown) => {
        console.log("setting new item");
        console.log("result", result);
        localStorage.setItem(fileName, (result as string).toUpperCase())
        return (result as string).toUpperCase();
      })
      .catch(e => null);
  }
}

async function fetchPatches() {
  
  try {
    patches = await invoke("get_patches");
    dlText!.innerHTML = `getting patch info`;
  } catch (error) {
    dlText!.innerHTML = `there seems to be a problem getting the patches: ${error}`;
    await message(`an error occurred!' ${error}`, {
      title: "Error",
      type: "error",
    });
  }
  downloadArray = [];
  console.log(patches);
  console.time("test_timer");

  for (const [index, patch] of patches.entries()) {
    setButtonState(ButtonStates.VERIFY, true);
    dlText!.innerHTML = `<div class="percent"> Verifying files...</div><div class="file">${patch.ObjectName}</div>  <div class="speed">Patch ${index + 1}/${patches.length}</div>`;
    let filePath = `${installDirectory}/Data/${patch.ObjectName}`;

    if (patch.ObjectName == "dusk-wow.exe") {
      filePath = `${installDirectory}/${patch.ObjectName}`;
    }

    const encoded = await getFileHash(filePath);
    console.log("file hash:",encoded);
    console.log("remote hash:", patch.Checksum)
    try {
      const timeStamp: { secs_since_epoch: number } = await invoke(
        "modified_time",
        { filePath }
      );
      if (
        (new Date(patch.LastChanged).getTime() / 1000 >
        timeStamp.secs_since_epoch) || encoded !== patch.Checksum
      ) {
        await downloadArray.push({ ...patch, filePath });
      }
    } catch (error) {
      console.log(error);
      await downloadArray.push({ ...patch, filePath });
    }
  }
  console.log(downloadArray);
  console.timeEnd("test_timer");
  if (downloadArray.length === 0) {
    dlText!.innerHTML = `ready to play`;
    setButtonState(ButtonStates.PLAY, false);
  }
  // else if (downloadArray.length === patches.length) {
  //   dlText!.innerHTML = `press download to install the custom duskhaven patches`;
  //   setButtonState(ButtonStates.DOWNLOAD, false);
  // }
  else {
    dlText!.innerHTML = `there is an update available`;
    setButtonState(ButtonStates.UPDATE, false);
  }
}

function setButtonState(state: ButtonStates, disabled: boolean) {
  playButton.disabled = disabled;
  directorySelector.disabled = disabled;
  if (statusText) statusText.innerHTML = state;
}

async function appClose() {
  if (playButton.disabled) {
    const confirmed = await ask(
      "Closing the launcher while downloading will corrupt the download. Are you sure you want to close?",
      { title: "Duskhaven Launcher", type: "warning" }
    );
    console.log(confirmed);
    if (confirmed) {
      appWindow.close();
    }
  } else {
    appWindow.close();
  }
}
async function getNews() {
  const newsList: HTMLElement = document.getElementById(
    "newslist"
  ) as HTMLElement;
  const client = await getClient();
  const options = {
    headers: {
      Authorization: `Bearer ${process.env.VITE_STRAPPI_TOKEN || import.meta.env.VITE_STRAPPI_TOKEN
        }`,
      "Content-Type": "application/json",
    },
    responseType: ResponseType.JSON,
  };

  const response = await client.get(
    `${process.env.VITE_STRAPPI_URL || import.meta.env.VITE_STRAPPI_URL
    }/blogs?pagination[page]=1&pagination[7]=1&populate=* `,
    options
  );
  const data: any = response.data;
  data.data.forEach((newsItem: any) => {
    let date = new Date(newsItem.attributes.updatedAt);

    let month = "" + (date.getUTCMonth() + 1),
      day = "" + date.getUTCDate(),
      year = date.getUTCFullYear();

    if (month.length < 2) month = "0" + month;
    if (day.length < 2) day = "0" + day;

    let formattedDate = [month, day, year].join("/");
    const newsNode = document.createElement("li");
    newsNode.innerHTML = `<a class="row" target="_blank" href="https://www.duskhaven.net/blog/${newsItem.id}"><div class="news_title"><span class="news_category ${newsItem.attributes.Category}">${newsItem.attributes.Category}</span> ${newsItem.attributes.Title} </div><div class="news_date">${formattedDate}</div></a>`;
    newsList.appendChild(newsNode);
  });
}

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
  document.body.style.backgroundImage = "url('/img/background.gif')";
});

function playAudio() {
  playSound.volume = 0.3;
  playSound.play();
}
