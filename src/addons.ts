import { invoke } from "@tauri-apps/api";

const elvUIButton = document.getElementById("elvui") as HTMLButtonElement;
let loadedAddons: String[] = [];

window.addEventListener("DOMContentLoaded", () => {
  getAddons();
  elvUIButton?.addEventListener("click", elvUI);
});

async function elvUI() {
  const installDirectory = localStorage.getItem("installDirectory");
  if (!loadedAddons.includes("ElvUI")) {
    invoke("download_files", {
      urls: ["https://github.com/ElvUI-WotLK/ElvUI/archive/refs/tags/6.09.zip"],
      destinations: [`${installDirectory}/downloads/ElvUI.zip`],
    })
      .then(() => {
        invoke("download_addon", {
          fileDirectory: `${installDirectory}/downloads/ElvUI.zip`,
          installDirectory: installDirectory,
        });
      })
      .catch((e) => console.log(e));
  } else {
    console.log("Already installed");
  }
}

async function getAddons() {
  const installDirectory = localStorage.getItem("installDirectory");
  invoke("get_addons", { installDirectory })
    .then((res) => {
      const response: String[] = res as any;
      loadedAddons = response;
      console.log(loadedAddons);
    })
    .catch((e) => {
      console.log(e);
    });
}
