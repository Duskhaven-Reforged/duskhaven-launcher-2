// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
extern crate serde_json;

use futures::io::Cursor;
use log::{error, info};
// use reqwest::header::HeaderMap;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;
use zip_extract::extract;
use std::io::{self, Error, Read};
//use std::path::Path;
use regex::{Captures, Regex};
use sha2::{Digest, Sha256};
// use regex::{Captures, Regex};
use std::process::Command;
use std::time::{Instant, SystemTime};
use std::{fs, panic};
use tauri::Manager;
use tokio::fs::File;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};

#[derive(Serialize)]
pub struct Progress {
    pub download_id: i64,
    pub filesize: u64,
    pub transfered: u64,
    pub transfer_rate: f64,
    pub percentage: f64,
}

pub type Patches = Vec<Patch>;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Patch {
    guid: String,
    storage_zone_name: String,
    path: String,
    object_name: String,
    length: i64,
    last_changed: String,
    server_id: i64,
    array_number: i64,
    is_directory: bool,
    user_id: String,
    content_type: String,
    date_created: String,
    storage_zone_id: i64,
    checksum: String,
    replicated_zones: String,
}

// const KEY53: u64 = 8186484168865098;
// const KEY14: u64 = 4887;

const addons: &'static [&'static str] = &["ElvUI", "Clique"];

#[tauri::command]
fn modified_time(file_path: String) -> Result<SystemTime, String> {
    modified_time_of(file_path).map_err(|err| err.to_string()) // separate function to change either error to String
}

fn modified_time_of(file_path: String) -> Result<SystemTime, std::io::Error> {
    let meta = fs::metadata(file_path)?;
    meta.modified()
}

#[tauri::command]
async fn sha256_digest(file_location: String) -> Result<String, String> {
    println!("{}", file_location);
    //get file
    let mut file = File::open(file_location)
        .await
        .map_err(|err| err.to_string())?;
    let mut buffer = Vec::new();
    //read file to end (reads it in binary)
    file.read_to_end(&mut buffer)
        .await
        .map_err(|err| err.to_string())?;
    //sets up a sha256 algorith to digest the file
    let mut context = Sha256::new();

    //adds content to hasher
    context.update(buffer);
    
    let digest = context.finalize();
    //hashes the file contents and sends it
    let digest_string = hex::encode(digest);
    Ok(digest_string)
}

#[tauri::command]
async fn get_patches() -> Result<Patches, String> {
    let client = Client::new();
    let body = client
        .get("https://storage.bunnycdn.com/duskhaven-patches/")
        .header("AccessKey", "e56f9198-3a9c-4e06-9be7cfec52c3-4757-4aac")
        .send()
        .await
        .map_err(|err| err.to_string())?;
    let b = body.text().await.unwrap();
    let patches: Patches = serde_json::from_str(&b).unwrap();

    Ok(patches)
}

#[tauri::command]
async fn get_addons(installDirectory: String) -> Result<Vec<String>, String> {
    let mut installed_addons: Vec<String> = vec![];
    for ele in addons {
        let directory = installDirectory.to_string() + "/Interface/AddOns/" + ele;
        if Path::new(&directory).exists() {
            installed_addons.push(ele.to_string());
        }
    }
    Ok(installed_addons)
}

#[tauri::command]
async fn download_addon(fileDirectory: String, installDirectory: String, topFolder: bool) -> Result<String, String> {
    let targetDir = PathBuf::from(format!("{}/Interface/AddOns", installDirectory));

    // Read the zip file from disk using spawn_blocking
    let read_result =  std::fs::read(fileDirectory);
    let archive = match read_result {
        Ok(archive) => archive,
        Err(_) => return Err("Issue in reading archive".to_string()),
    };

    // Extract the zip file to another folder using spawn_blocking
    let extract_result = spawn_blocking(move || {
        let cursor = std::io::Cursor::new(archive);
        extract(cursor, &targetDir, topFolder)
    }).await;

    if let Err(_) = extract_result {
        return Err("Issue in extracting archive".to_string());
    }

    Ok("Success".to_string())
}

#[tauri::command]
fn delete_addon(addonName: String, installDirectory: String) -> Result<String, String> {
    let addonPath = format!("{}/Interface/AddOns/{}", installDirectory, addonName);
    match std::fs::remove_dir_all(addonPath) {
        Ok(_) => Ok("Deletion successful".to_string()),
        Err(e) => Err(format!("Error deleting addon: {}", e)),
    }
}

#[tauri::command]
fn open_app(path: String) -> Result<String, String> {
    let child = Command::new(path).spawn().map_err(|err| err.to_string())?;

    Ok(format!("Application opened with PID {}", child.id()))
}

// fn inv256() -> Vec<u64> {
//     let mut inv256 = vec![0; 128];
//     for m in 0..128 {
//         let mut inv = 1;
//         loop {
//             inv += 2;
//             if inv * ((2 * m + 1) as u64) % 256 == 1 {
//                 break;
//             }
//         }
//         inv256[m] = inv;
//     }
//     inv256
// }

// fn encode(str: &str) -> String {
//     let inv256 = inv256();
//     let mut k = KEY53;
//     let f = 16384 + KEY14;
//     str.chars()
//         .map(|m| {
//             let m = m as u64;
//             let l = k % 274877906944;
//             let h = (k - l) / 274877906944;
//             let mm = h % 128;
//             let c = (m * inv256[mm as usize] - (h - mm as u64) / 128) % 256;
//             k = l * f + h + c + m;
//             format!("{:02x}", c)
//         })
//         .collect()
// }

#[tauri::command]
async fn download_files(
    app: tauri::AppHandle,
    urls: Vec<String>,
    destinations: Vec<String>,
) -> Result<(), String> {
    let client = Client::new();
    if urls.is_empty() {
        return Err("No urls to process".to_string());
    }

    for (index, url) in urls.iter().enumerate() {
        let destination = &destinations[index];

        let total_size = client
            .head(url)
            .send()
            .await
            .map_err(|err| err.to_string())?;

        if total_size.status().is_success() {
            let size = total_size
                .headers()
                .get(reqwest::header::CONTENT_LENGTH)
                .and_then(|ct_len| ct_len.to_str().ok().and_then(|ct_len| ct_len.parse().ok()))
                .unwrap_or(0);

            let request = client.get(url);
            let mut response = request.send().await.map_err(|err| err.to_string())?;

            let mut out = BufWriter::new(
                File::create(&destination)
                    .await
                    .map_err(|err| err.to_string())?,
            );
            let mut downloaded: u64 = 0;
            let start = Instant::now();
            let mut progress = Progress {
                download_id: index as i64,
                filesize: size,
                transfered: 0,
                transfer_rate: 0.0,
                percentage: 0.0,
            };

            while let Some(chunk) = response.chunk().await.map_err(|err| err.to_string())? {
                match out.write_all(&chunk).await {
                    Ok(_) => (),
                    Err(err) => {
                        error!("the problem is :{}", err.to_string());
                        println!("the problem is :{}", err.to_string());
                        return Err(err.to_string());
                    }
                };
                downloaded += chunk.len() as u64;
                out.flush().await.map_err(|err| err.to_string())?;
                progress.transfered = downloaded;
                progress.percentage = if size != 0 {
                    (100.0 * downloaded as f64) / size as f64
                } else {
                    0.0
                };
                progress.transfer_rate = downloaded as f64 / start.elapsed().as_secs_f64();

                match app.emit_all("DOWNLOAD_PROGRESS", &progress) {
                    Ok(_) => {
                        //println!("the progress is :{}%", progress.percentage.to_string());
                    }
                    Err(err) => {
                        println!("the problem is :{}", err.to_string());
                        return Err(err.to_string());
                    }
                };
            }
            match app.emit_all("DOWNLOAD_FINISHED", &progress) {
                Ok(_) => { info!("download for file {} finished", &destination)}
                Err(err) => {
                    println!(
                        "the problem is :{}",
                        total_size.status().as_str().to_string()
                    );
                    return Err(err.to_string());
                }
            }
            // app.emit_all("DOWNLOAD_FINISHED", &progress).unwrap();
        } else {
            return Err(total_size.status().as_str().to_string());
        }
    }
    Ok(())
}

fn main() {
    panic::set_hook(Box::new(|panic_info| {
        if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            println!("Panic occurred: {}", s);
        } else {
            println!("Panic occurred");
        }
    }));
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            download_files,
            get_patches,
            modified_time,
            open_app,
            get_addons,
            download_addon,
            delete_addon,
            sha256_digest
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    info!("Starting application");
}
