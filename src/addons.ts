import { invoke } from "@tauri-apps/api";

let loadedAddons: String[] = [];
const allAddons: Record<string, string> = {
  ElvUI: "https://github.com/ElvUI-WotLK/ElvUI/archive/refs/tags/6.09.zip",
  Clique: "https://legacy-wow.com/uploads/addons/wotlk/Clique-3.3.5.zip",
};

window.addEventListener("DOMContentLoaded", () => {
  getAddons();
});

async function downloadAddon(addonName: string, dlUrl: string) {
  const installDirectory = localStorage.getItem("installDirectory");
  const stripTopFolder: boolean = addonName === "ElvUI";
  const button = document.querySelector(`#${addonName}-button`);
  if (!button) {
    return;
  }
  button.innerHTML = '<iconify-icon icon="tabler:loader"></iconify-icon>';

  if (!loadedAddons.includes(addonName)) {
    invoke("download_files", {
      urls: [dlUrl],
      destinations: [`${installDirectory}/downloads/${addonName}.zip`],
    })
      .then(() => {
        invoke("download_addon", {
          fileDirectory: `${installDirectory}/downloads/${addonName}.zip`,
          installDirectory: installDirectory,
          topFolder: stripTopFolder,
        }).then(() => {
          getAddons();
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
      updateAddonState();
    })
    .catch((e) => {
      console.log(e);
    });
}

async function updateAddonState() {
  const container = document.querySelector("#addonList");

  if (!container) {
    return;
  }

  container.innerHTML = "";

  console.log(allAddons);

  Object.keys(allAddons).forEach((addonName) => {
    const listItem = document.createElement("li");
    listItem.classList.add("addon");
    const alreadyInstalled: boolean = loadedAddons.includes(addonName);
    let buttonToAdd = alreadyInstalled
      ? `<button id="${addonName}-button">
        <iconify-icon icon="material-symbols:delete-outline"></iconify-icon>
      </button>`
      : `<button id="${addonName}-button">
        <iconify-icon icon="material-symbols:download"></iconify-icon>
      </button>`;
    listItem.innerHTML = `
      ${addonName} ${buttonToAdd}
    `;

    // Add an event listener to the button if needed
    const button = listItem.querySelector(`#${addonName}-button`);
    if (button) {
      alreadyInstalled
        ? button.addEventListener("click", () => {
            // Handle click event
            deleteAddon(addonName);
          })
        : button.addEventListener("click", () => {
            // Handle click event
            downloadAddon(addonName, allAddons[addonName]);
          });
    }

    container.appendChild(listItem);
  });
}

async function deleteAddon(addonName: string) {
  const installDirectory = localStorage.getItem("installDirectory");
  invoke("delete_addon", {
    addonName: addonName,
    installDirectory: installDirectory,
  })
    .then(() => getAddons())
    .catch((e) => console.log(e));
}
