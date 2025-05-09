import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { appDataDir } from "@tauri-apps/api/path";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { ask, open } from "@tauri-apps/plugin-dialog";
import { exists, readDir } from "@tauri-apps/plugin-fs";
import { fetch } from "@tauri-apps/plugin-http";
import Swal from "sweetalert2";
import { Patch, Progress } from "./patch";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

const appWindow = getCurrentWebviewWindow();
// import i18n from "./i18n";

enum ButtonStates {
  PLAY = "Play",
  DOWNLOAD = "Download",
  UPDATE = "Update",
  VERIFY = "Verifying",
}

// Define the log message object
interface LogMessage {
  message: string;
  level: "error" | "warn" | "info";
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
const dlSpeed: HTMLElement | null = document.querySelector(".download-speed");
const dlText: HTMLElement | null = document.querySelector(
  ".download-container .text-center"
);
window.addEventListener("DOMContentLoaded", async () => {
  const update = await check();
  if (update?.available) {
    showUpdatePrompt(update);
  }
  const hasAccess = await checkAccess();
  document
    .querySelector("[data-tauri-drag-region]")
    ?.addEventListener("mousedown", (e) => {
      // Get the clicked element
      const target = e.target as HTMLElement;

      // Check if the clicked element or its parents have specific roles or tags
      const isInteractive = target.closest(
        'button, a, input, [role="button"], #titlebar-minimize, #titlebar-fix, #titlebar-close, #titlebar-dir, #animation-toggle'
      );

      // Only start dragging if we're not clicking an interactive element
      if (!isInteractive) {
        appWindow.startDragging();
      }
    });
  document
    .getElementById("titlebar-minimize")
    ?.addEventListener("click", () => appWindow.minimize());
  document
    .getElementById("titlebar-fix")
    ?.addEventListener("click", () => removeCache());
  document
    .getElementById("titlebar-dir")
    ?.addEventListener("click", () => setInstallDirectory());
  document
    .getElementById("titlebar-close")
    ?.addEventListener("click", () => appClose());
  document
    .getElementById("animation-toggle")
    ?.addEventListener("click", toggleAnimation);
  autoPlayCheck.checked = autoPlay;
  if(hasAccess) {
    hasInstallDirectory();
    document.getElementById("autoplay")?.addEventListener("change", setAutoPlay);
    playButton?.addEventListener("click", handlePlayButton);
    getNews();
  } else {
    appWindow.close();
  }
  
});

async function isValidWowFolder(folder: string): Promise<boolean> {
  try {
    const files = await readDir(folder);
    return files.some((file) => file.name.toLowerCase() === "common.mpq");
  } catch (e) {
    await Swal.fire({
      title: "Installation Directory",
      text: "It seems you have no Data folder or correct install folder for your 3.3.5 client.",
      showCancelButton: false,
      heightAuto: false,
      animation: false,
      confirmButtonText: "Select Folder",
      allowOutsideClick: false,
      allowEscapeKey: false,
    });
    console.error("Error reading directory:", e);
    return false;
  }
}

async function showUpdatePrompt(update: any) {
  const result = await Swal.fire({
    title: `v${update. version} Update Available!`,
    text: `An update is available. Would you like to update now? you are currently on version ${update.currentVersion} and the latest version is ${update.version}`,  
    icon: "info",
    showCancelButton: true,
    confirmButtonText: "Ok",
    cancelButtonText: "Cancel",
  });
  
  if (result.isConfirmed) {
    await update.downloadAndInstall();
    await relaunch();
  } else {
    console.log("User canceled update.");
    // Do nothing
  }
}

async function checkAccess() {
  console.log("Checking access to the server...", process.env.VITE_API_BASE_URL, import.meta.env.VITE_API_BASE_URL);
  const response = await fetch(`${process.env.VITE_API_BASE_URL || import.meta.env.VITE_API_BASE_URL}launcher/restricted-check`);
  const { restrictedMode } = await response.json();
  console.log("Restricted mode:", restrictedMode);
  if (restrictedMode) {
    const { value: username } = await Swal.fire({
      title: "Restricted Access",
      text: "Server is only available to authorized QA members and Developers. Please enter your username to verify access. If you are not part of QA or Development, you will not be able to proceed.",
      input: "text",
      inputPlaceholder: "Enter your username",
      showCancelButton: false,
      confirmButtonText: "Submit",
      allowOutsideClick: false,
      allowEscapeKey: false,
    });

    if (username) {
      const accessResponse = await fetch(`${ process.env.VITE_API_BASE_URL || import.meta.env.VITE_API_BASE_URL}launcher/check-access`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ username }),
      });

      const { allowed } = await accessResponse.json();

      if (!allowed) {
        await Swal.fire({
          title: "Access Denied",
          text: "You are not allowed to download files or access the server at this point.",
          icon: "error",
          confirmButtonText: "Ok",
        });
        return false;// Close the launcher if access is denied
      }
      else {
        await Swal.fire({
          title: "Access Granted for now",
          icon: "success",
          confirmButtonText: "Ok",
        });
      }
    } else {
      return false;
    }
  } else {
    return true; // Access is granted, do nothing
  }
  return true; // Default to true if no conditions are met

}

async function hasInstallDirectory() {
  const validWowFolder = await isValidWowFolder(`${installDirectory}/data`);

  if (installDirectory && validWowFolder) {
    fetchPatches();
  } else {
    await Swal.fire({
      title: "Installation Directory",
      text: "Please select the folder containing your valid 3.3.5 WoW client.",
      showCancelButton: false,
      heightAuto: false,
      animation: false,
      confirmButtonText: "Select Folder",
      allowOutsideClick: false,
      allowEscapeKey: false,
    });
    setInstallDirectory();
  }
}

async function setInstallDirectory() {
  const appdir = await appDataDir();
  
  await logMessage(`Setting install directory to ${appdir}`, "info");
  let valid = false;
  let selected: string | null | string[];

  do {
    selected = await open({
      directory: true,
      multiple: false,
      defaultPath: installDirectory || appdir,
    });

    if (Array.isArray(selected)) {
      installDirectory = selected[0]!;
    } else if (selected === null) {
      await logMessage(`Setting of the directory canceled`, "warn");
      // user cancelled the selection
    } else {
      installDirectory = selected;
    }

    if (await exists(`${installDirectory}/Data/patch-3.MPQ`)) {
      valid = true;
      await logMessage(
        `Local storage will now remember your directory as ${installDirectory}`,
        "info"
      );
      localStorage.setItem("installDirectory", installDirectory!);
      fetchPatches();
    } else {
      await logMessage(
        `"patch-3.MPQ" not found in the selected directory. Please select again.`,
        "error"
      );
      await Swal.fire({
        title: "Invalid Directory",
        text: `"${installDirectory}" is not a valid 3.3.5 client folder. Please select a valid directory.`,
        showCancelButton: false,
        icon: "error",
        heightAuto: false,
        animation: false,
        confirmButtonText: "ok",
        allowOutsideClick: false,
        allowEscapeKey: false,
      });
      valid = false; // Prompt the user to select again
    }
    await logMessage(
      `Local storage will now remember your directory as ${installDirectory}`,
      "info"
    );
    localStorage.setItem("installDirectory", installDirectory!);
  } while (!valid);
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

function removeCache() {
  localStorage.clear();
  location.reload();
}

function setAutoPlay(e: any) {
  localStorage.setItem("autoPlay", e.target.checked);
}

// listen for progress updates
listen("DOWNLOAD_PROGRESS", (event) => {
  const progress: any = event.payload;

  dlProgress!.style!.width = `${progress.percentage}%`;
  dlSpeed!.innerHTML = `${(progress.transfer_rate / 1000 / 1000).toFixed(
    2
  )} MB/sec`;
  dlText!.innerHTML = `<div class="percent"> ${progress.percentage.toFixed(
    2
  )}%</div><div class="file">${
    downloadArray[progress.download_id].ObjectName
  } </div> `;
});

// listen for download finished
listen("DOWNLOAD_FINISHED", (event: { payload: Progress }) => {
  console.log(event.payload.filesize);
});

async function startGame() {
  playAudio();
  invoke("open_app", { path: `${installDirectory}/wow.exe` })
    .then(() => setTimeout(appClose, 5000))
    .catch(async (error) => {
      await Swal.fire({
        title: "Something went wrong",
        text: `Seems we can't start world of warcraft. Check your permissions for this folder and wow.exe. more info: ${error}`,
        showCancelButton: false,
        icon: "error",
        heightAuto: false,
        animation: false,
        confirmButtonText: "ok",
        allowOutsideClick: false,
        allowEscapeKey: false,
      });
    });
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
      .then(async () => {
        setButtonState(ButtonStates.VERIFY, true);
        for (const file of destinations) {
          const fileName = file?.split("/").pop()!;
          dlText!.innerHTML = `<div class="percent"> Verifying file: </div><div class="file">${fileName}</div> `;
          await getFileHash(file!, true);
        }
        dlProgress!.style!.width = `100%`;
        dlSpeed!.innerHTML = "";
        dlText!.innerHTML = `Ready to play!`;
        if (autoPlayCheck.checked) {
          startGame();
        }
        setButtonState(ButtonStates.PLAY, false);
      })
      .catch(async (err) => {
        setButtonState(ButtonStates.UPDATE, false);
        await Swal.fire({
          title: "Something went wrong",
          text: `Something went wrong while downloading the files.  ${err}`,
          showCancelButton: false,
          icon: "error",
          heightAuto: false,
          animation: false,
          confirmButtonText: "ok",
          allowOutsideClick: false,
          allowEscapeKey: false,
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

async function getFileHash(fileLocation: string, forced = false) {
  const fileName = fileLocation.split("/").pop()!;

  return await invoke("sha256_digest", {
    fileLocation,
    localHash: localStorage.getItem(fileName) || "",
    forced,
  })
    .then(async (result: unknown) => {
      await logMessage(
        `New hash for file set: ${(result as string).toUpperCase()}`,
        "info"
      );
      localStorage.setItem(fileName, (result as string).toUpperCase());
      return (result as string).toUpperCase();
    })
    .catch((e) => console.log("File doesn't exist", e));

  // if (localStorage.getItem(fileName) && !force) {
  //   return localStorage.getItem(fileName);
  // } else {

  // }
}

async function fetchPatches() {
  try {
    const patchesPlain: string = await invoke("get_patches");
    await logMessage(
      `Getting all the patches ${JSON.parse(patchesPlain)}`,
      "info"
    );
    patches = JSON.parse(patchesPlain);
    patches = patches.filter((value) => value.IsDirectory === false);
    dlText!.innerHTML = `Fetching patch information...`;
  } catch (error) {
    dlText!.innerHTML = `There was a problem retrieving the patches`;
    await logMessage(
      `Getting all the patches went sideways: ${error}`,
      "error"
    );
    await Swal.fire({
      title: "Something went wrong",
      text: `Something went wrong with fetching the patches. ${error}`,
      showCancelButton: false,
      icon: "error",
      heightAuto: false,
      animation: false,
      confirmButtonText: "ok",
      allowOutsideClick: false,
      allowEscapeKey: false,
    });
  }
  downloadArray = [];

  for (const [index, patch] of patches.entries()) {
    setButtonState(ButtonStates.VERIFY, true);
    dlSpeed!.innerHTML = `Patch ${index + 1}/${patches.length}`;
    dlText!.innerHTML = `<div class="percent"> Verifying files...</div><div class="file">${patch.ObjectName}</div> `;
    let filePath = `${installDirectory}/${patch.ObjectName}`;

    if (patch.ObjectName.toLowerCase().includes(".mpq")) {
      filePath = `${installDirectory}/Data/${patch.ObjectName}`;
    }

    if (patch.ObjectName == "realmlist.wtf") {
      filePath = `${installDirectory}/Data/enUS/${patch.ObjectName}`;
    }
    const encoded = await getFileHash(filePath);

    await logMessage(`File hash: ${encoded}`, "info");
    await logMessage(`Remote file hash: ${patch.Checksum}`, "info");
    try {
      if (encoded !== patch.Checksum) {
        await downloadArray.push({ ...patch, filePath });
      }
    } catch (error) {
      await logMessage(`Remote file hash: ${JSON.stringify(error)}`, "error");
      await downloadArray.push({ ...patch, filePath });
    }
  }
  await logMessage(
    `Files to download: ${JSON.stringify(downloadArray)}`,
    "info"
  );
  if (downloadArray.length === 0) {
    dlText!.innerHTML = `Ready to play!`;
    dlSpeed!.innerHTML = `Ready to play!`;
    dlProgress!.style!.width = `100%`;
    setButtonState(ButtonStates.PLAY, false);
  }
  // else if (downloadArray.length === patches.length) {
  //   dlText!.innerHTML = `press download to install the custom duskhaven patches`;
  //   setButtonState(ButtonStates.DOWNLOAD, false);
  // }
  else {
    dlText!.innerHTML = `An update is available.`;
    setButtonState(ButtonStates.UPDATE, false);
  }
}

function setButtonState(state: ButtonStates, disabled: boolean) {
  playButton.disabled = disabled;
  if (statusText) statusText.innerHTML = state;
}

async function logMessage(message: string, level: "error" | "warn" | "info") {
  const log: LogMessage = { message, level };
  try {
    await invoke("log_message", { log });
  } catch (error) {
    console.error(`Failed to log message: ${message}`, error);
  }
}

async function appClose() {
  if (playButton.disabled) {
    const confirmed = await ask(
      "Are you sure you want to close the launcher? Closing it while a download is in progress may corrupt the download.",
      { title: "Duskhaven Launcher", kind: "warning" }
    );
    await logMessage(`Opening client`, "info");
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
  const options = {
    method: "GET",
    headers: {
      Authorization: `Bearer ${
        process.env.VITE_STRAPPI_TOKEN || import.meta.env.VITE_STRAPPI_TOKEN
      }`,
      "Content-Type": "application/json",
    },
  };

  const response = await fetch(
    `${
      process.env.VITE_STRAPPI_URL || import.meta.env.VITE_STRAPPI_URL
    }/blogs?pagination[page]=1&pagination[pageSize]=3&sort[0]=id:desc`,
    options
  );

  // Convert ReadableStream to text
  const reader = response.body?.getReader();
  if (!reader) {
    throw new Error("ReadableStream not available");
  }

  let jsonString = "";
  const decoder = new TextDecoder();

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    jsonString += decoder.decode(value);
  }
  const data = JSON.parse(jsonString); // Parse JSON

  data.data.forEach((newsItem: any) => {
    let date = new Date(newsItem.attributes.updatedAt);

    let month = "" + (date.getUTCMonth() + 1),
      day = "" + date.getUTCDate(),
      year = date.getUTCFullYear();

    if (month.length < 2) month = "0" + month;
    if (day.length < 2) day = "0" + day;

    let formattedDate = [month, day, year].join("/");
    const newsNode = document.createElement("li");
    newsNode.innerHTML = `<a class="row" target="_blank" href="https://duskhaven.net/blog/${
      newsItem.id
    }"><div class="news_title"><span class="news_category ${newsItem.attributes.Category.replace(
      /\s/g,
      ""
    )}">${newsItem.attributes.Category}</span> ${
      newsItem.attributes.Title
    } </div><div class="news_date">${formattedDate}</div></a>`;
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


listen("HASH_PROGRESS", (event) => {
  const progress: any = event.payload;

  dlProgress!.style!.width = `${progress.progress}%`;
  
  dlSpeed!.innerHTML = ``;
  dlText!.innerHTML = `<div class="percent"> ${progress.progress.toFixed(
    2
  )}%</div><div class="file">verifying file:${
    progress.filename
  } </div> `;
});
