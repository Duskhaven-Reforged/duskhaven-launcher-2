// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
extern crate serde_json;

use log::{error, info};
// use reqwest::header::HeaderMap;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
// use regex::{Captures, Regex};
use std::path::Path;
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

#[derive(Serialize, Deserialize)]
struct LogMessage {
    message: String,
    level: String,
}

// const KEY53: u64 = 8186484168865098;
// const KEY14: u64 = 4887;

#[tauri::command]
fn log_message(log: LogMessage) {
    match log.level.as_str() {
        "error" => log::error!("{}", log.message),
        "warn" => log::warn!("{}", log.message),
        "info" => log::info!("{}", log.message),
        _ => log::debug!("{}", log.message),
    }
}

#[tauri::command]
fn modified_time(file_path: String) -> Result<SystemTime, String> {
    info!("getting modified time of: {}", file_path);
    modified_time_of(file_path).map_err(|err| {
        error!("{}", err.to_string());
        err.to_string()
    }) // separate function to change either error to String
}

fn modified_time_of(file_path: String) -> Result<SystemTime, std::io::Error> {
    let mut destination = file_path;
    if destination.contains("enUS") {
        destination = get_correct_realmlist_path(&destination);
        println!("{}", destination);
    }
    let meta = fs::metadata(destination)?;
    meta.modified()
}

#[tauri::command]
async fn sha256_digest(file_location: String) -> Result<String, String> {
    info!("getting sha256_digest of file {}", file_location);

    let mut destination = file_location;
    if destination.contains("enUS") {
        destination = get_correct_realmlist_path(&destination);
        println!("{}", destination);
    }
    //get file
    let mut file = File::open(destination).await.map_err(|err| {
        error!("{}", err.to_string());
        err.to_string()
    })?;
    let mut buffer = Vec::new();
    //read file to end (reads it in binary)
    file.read_to_end(&mut buffer).await.map_err(|err| {
        error!("{}", err.to_string());
        err.to_string()
    })?;
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
async fn get_patches() -> Result<String, String> {
    info!("getting all patches available");
    let client = Client::new();
    let body = client
        .get("https://storage.bunnycdn.com/duskhaven-patches/")
        .header("AccessKey", "e56f9198-3a9c-4e06-9be7cfec52c3-4757-4aac")
        .send()
        .await
        .map_err(|err| {
            error!("{}", err.to_string());
            err.to_string()
        })?;
    let b = body.text().await.unwrap();
    //let patches: Patches = serde_json::from_str(&b).unwrap();

    Ok(b.to_string())
}

#[tauri::command]
fn open_app(path: String) -> Result<String, String> {
    info!("opening world of warcraft");
    let child = Command::new(path).spawn().map_err(|err| {
        error!("{}", err.to_string());
        err.to_string()
    })?;

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
    info!("starting to download files");
    let client = Client::new();
    if urls.is_empty() {
        info!("No urls to process");
        return Err("No urls to process".to_string());
    }

    for (index, url) in urls.iter().enumerate() {
        let mut destination = destinations[index].clone();

        let total_size = client.head(url).send().await.map_err(|err| {
            error!("{}", err.to_string());
            err.to_string()
        })?;

        if total_size.status().is_success() {
            let size = total_size
                .headers()
                .get(reqwest::header::CONTENT_LENGTH)
                .and_then(|ct_len| ct_len.to_str().ok().and_then(|ct_len| ct_len.parse().ok()))
                .unwrap_or(0);

            let request = client.get(url);
            let mut response = request.send().await.map_err(|err| {
                error!("{}", err.to_string());
                err.to_string()
            })?;
            println!("{}", destination);
            if destination.to_lowercase().contains("enus") {
                println!("WE GO GURL");
                destination = get_correct_realmlist_path(&destination);
                println!("{}", destination)
            }

            let mut out = BufWriter::new(File::create(&destination).await.map_err(|err| {
                error!("{}", err.to_string());
                err.to_string()
            })?);
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
                        error!("the problem is :{}", err.to_string());
                        println!("the problem is :{}", err.to_string());
                        return Err(err.to_string());
                    }
                };
            }
            match app.emit_all("DOWNLOAD_FINISHED", &progress) {
                Ok(_) => {
                    info!("download for file {} finished", &destination)
                }
                Err(err) => {
                    error!("the problem is :{}", err.to_string());
                    println!(
                        "the problem is :{}",
                        total_size.status().as_str().to_string()
                    );
                    return Err(err.to_string());
                }
            }
            // app.emit_all("DOWNLOAD_FINISHED", &progress).unwrap();
        } else {
            error!(
                "the problem is :{}",
                total_size.status().as_str().to_string()
            );
            return Err(total_size.status().as_str().to_string());
        }
    }
    info!("all files downloaded successfully");
    Ok(())
}

fn get_correct_realmlist_path(install_directory: &str) -> String {
    // Define possible language/region codes
    let language_codes = ["enUS", "enGB", "frFR", "deDE"]; // Add more as needed

    let base_directory = Path::new(install_directory)
        .parent() // This removes the filename `realmlist.wtf`
        .map(|p| p.to_string_lossy().to_string()) // Convert to String
        .unwrap_or_else(|| install_directory.to_string());

    // Extract the filename separately so we can add it back later
    let file_name = Path::new(install_directory)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // Iterate over possible directories
    for code in language_codes.iter() {
        let modified = base_directory.replace("enUS", code);
        println!("{}", modified);
        if Path::new(&modified).exists() {
            let full_path = Path::new(&modified).join(file_name);
            return full_path.to_string_lossy().to_string();
        }
    }

    // Fallback to a default path if none found
    let fallback_path = format!("{}", install_directory);
    fallback_path
}

fn setup_logging() -> Result<(), fern::InitError> {
    info!("setting up logging");
    // Customize the log file location here
    let log_file_path = "logs/my_app.log"; // You can change this path
    std::fs::create_dir_all("logs").expect("Failed to create log directory");
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d %H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        // Output to stdout
        //.chain(std::io::stdout())
        // Output to a file
        .chain(fern::log_file(log_file_path)?)
        .apply()?;

    info!("logging set up");
    Ok(())
}

fn main() {
    setup_logging().expect("failed to initialize logging");
    panic::set_hook(Box::new(|panic_info| {
        if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            error!("PANIC!: {}", s);
        } else {
            error!("PANIC!");
        }
    }));
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            download_files,
            get_patches,
            modified_time,
            open_app,
            sha256_digest,
            log_message
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
