// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
extern crate serde_json;

use reqwest::header::HeaderMap;
// src-tauri/src/main.rs
use hex;
use hex::encode;
use reqwest::{Client, RequestBuilder};
use ring::digest::{Context, Digest, SHA256};
use serde::{Deserialize, Serialize};
use std::io::{Error, Read};
use std::path::Path;
use std::time::{Instant, SystemTime};
use std::{fs, panic};
use tauri::Manager;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};

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

#[tauri::command]
fn modified_time(file_path: String) -> Result<SystemTime, String> {
    modified_time_of(file_path).map_err(|err| err.to_string()) // separate function to change either error to String
}

fn modified_time_of(file_path: String) -> Result<SystemTime, std::io::Error> {
    let meta = fs::metadata(file_path)?;
    meta.modified()
}

fn sha256_digest<R: Read>(mut reader: R) -> Result<Digest, Error> {
    let mut context = Context::new(&SHA256);
    let mut buffer = [0; 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        context.update(&buffer[..count]);
    }
    Ok(context.finish())

    // How to Use:
    // let path = "C:/Games/patch-J.mpq";
    // let mut file = File::open(path).await.map_err(|e| e.to_string())?;
    // let mut buffer = Vec::new();
    // file.read_to_end(&mut buffer)
    //     .await
    //     .map_err(|e| e.to_string())?;
    // let digest = sha256_digest(&mut buffer.as_slice()).unwrap();
    // let digest_string = encode(digest.as_ref());
    // println!("SHA-256 digest is {}", digest_string);
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
                        println!("the problem is :{}%", err.to_string());
                        return Err(err.to_string());
                    }
                };
                downloaded += chunk.len() as u64;

                progress.transfered = downloaded;
                progress.percentage = if size != 0 {
                    (100.0 * downloaded as f64) / size as f64
                } else {
                    0.0
                };
                progress.transfer_rate = downloaded as f64 / start.elapsed().as_secs_f64();
                //println!("the progress is :{}%", progress.percentage.to_string());
                match app.emit_all("DOWNLOAD_PROGRESS", &progress) {
                    Ok(_) => {
                        println!("the progress is :{}%", progress.percentage.to_string());
                    }
                    Err(err) => {
                        println!("the problem is :{}%", err.to_string());
                        return Err(err.to_string());
                    }
                };
            }
            match app.emit_all("DOWNLOAD_FINISHED", &progress) {
                Ok(_) => {}
                Err(err) => {
                    println!(
                        "the problem is :{}%",
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
            modified_time
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
