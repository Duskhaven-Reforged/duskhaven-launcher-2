// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
extern crate serde_json;

// use reqwest::header::HeaderMap;
// src-tauri/src/main.rs
// use hex::encode;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::io::{Error, Read};
use sha2::{Sha256, Digest};
//use std::path::Path;
use regex::{Captures, Regex};
use std::process::Command;
use std::time::{Instant, SystemTime};
use std::{fs, panic};
use tauri::Manager;
use tokio::fs::File;
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

const KEY53: u64 = 8186484168865098;
const KEY14: u64 = 4887;

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
    let mut file = File::open(file_location).await.map_err(|err| err.to_string())?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).await.map_err(|err| err.to_string())?;
    let reader = &mut buffer.as_slice();

    let mut context = Sha256::new();
    let mut buffer = [0;  1024];
    loop {
        let count = std::io::Read::read(reader, &mut buffer).map_err(|err| err.to_string())?;
        if count ==  0 {
            break;
        }
        context.update(&buffer[..count]);
    }
    let digest = context.finalize();
    let digest_string = hex::encode(digest); // Use hex::encode to convert the digest to a string
    Ok(digest_string)

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
fn open_app(path: String) -> Result<String, String> {
    let child = Command::new(path).spawn().map_err(|err| err.to_string())?;

    Ok(format!("Application opened with PID {}", child.id()))
}

#[tauri::command]
fn update_account_info(
    installDirectory: String,
    username: String,
    password: String,
) -> Result<String, String> {
    // Define the regex pattern
    let file_path = format!("{}/WTF/Config.wtf", installDirectory);
    let re = Regex::new(r#"(?m)^SET accountName .*$"#).unwrap();

    // Read the file
    let mut contents = match fs::read_to_string(&file_path) {
        Ok(contents) => contents,
        Err(e) => return Err(e.to_string()),
    };

    // Check if the line exists
    if re.captures(&contents).is_none() {
        // Append the line to the end of the file
        contents.push_str(&format!("SET accountName \"{} {}\"\n", username, encode(&password)));
    } else {
        // Replace the matched line
        contents = re
            .replace(&contents, |caps: &Captures| {
                format!("SET accountName \"{} {}\"", username, encode(&password))
            })
            .to_string();
    }

    // Write the new contents back to the file
    match fs::write(&file_path, contents) {
        Ok(_) => Ok(format!("Application opened with PID ")),
        Err(e) => Err(e.to_string()),
    }
}

fn inv256() -> Vec<u64> {
    let mut inv256 = vec![0; 128];
    for M in 0..128 {
        let mut inv = 1;
        loop {
            inv += 2;
            if inv * ((2 * M + 1) as u64) % 256 == 1 {
                break;
            }
        }
        inv256[M] = inv;
    }
    inv256
}

fn encode(str: &str) -> String {
    let inv256 = inv256();
    let mut K = KEY53;
    let F = 16384 + KEY14;
    str.chars().map(|m| {
        let m = m as u64;
        let L = K % 274877906944;
        let H = (K - L) / 274877906944;
        let M = H % 128;
        let c = (m * inv256[M as usize] - (H - M as u64) / 128) % 256;
        K = L * F + H + c + m;
        format!("{:02x}", c)
    }).collect()
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
                out.flush().await.map_err(|err| err.to_string())?;
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
            modified_time,
            open_app,
            update_account_info,
            sha256_digest
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
