// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
extern crate serde_json;

use log::{error, info};
use tauri::{AppHandle, Emitter};
// use reqwest::header::HeaderMap;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
// use regex::{Captures, Regex};
use std::panic;
use std::path::Path;
use std::process::Command;
use std::time::Instant;
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

#[derive(Serialize, Clone)]
struct HashProgress {
    progress: f64,
    filename: String,
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
async fn sha256_digest(
    app: AppHandle,
    file_location: String,
    local_hash: String,
    forced: bool,
) -> Result<String, String> {
    info!("getting sha256_digest of file {}", file_location);

    let mut destination = file_location;
    if destination.contains("enUS") {
        destination = get_correct_realmlist_path(&destination);
        println!("{}", destination);
    }
    //get file
    let mut file = File::open(&destination)
        .await
        .map_err(|err| err.to_string())?;

    // Retrieve the file size
    let metadata = file.metadata().await.map_err(|err| err.to_string())?;
    let expected_file_size = metadata.len();

    // If a local hash is provided, return it immediately
    if !forced && !local_hash.is_empty() {
        info!("Using provided local hash: {}", local_hash);
        return Ok(local_hash);
    }

    // Get the file name from the destination path.
    let file_name = Path::new(&destination)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // Define maximum allowed total read size and the chunk size.
    const CHUNK_SIZE: usize = 16 * 1024 * 1024; // 8MB per chunk

    let mut context = Sha256::new();
    let mut total_read: u64 = 0;
    let mut buffer = vec![0u8; CHUNK_SIZE];
    //read file to end (reads it in binary)
    loop {
        let n = file
            .read(&mut buffer)
            .await
            .map_err(|err| err.to_string())?;
        if n == 0 {
            break;
        }
        total_read += n as u64;
        context.update(&buffer[..n]);

        let progress = total_read as f64 / (expected_file_size as f64) * 100.0;
        let progress = HashProgress {
            progress: progress,
            filename: file_name.clone(),
        };
        app.emit("HASH_PROGRESS", progress)
            .map_err(|e| e.to_string())?;
    }

    let digest = context.finalize();
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
    app: AppHandle,
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
                //app.emit("DOWNLOAD_PROGRESS", &progress).unwrap();
                match app.emit("DOWNLOAD_PROGRESS", &progress) {
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
            match app.emit("DOWNLOAD_FINISHED", &progress) {
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
    // Get the directory of the current executable
    let exe_dir = std::env::current_exe()
        .expect("Failed to determine the executable path")
        .parent()
        .expect("Executable does not have a parent directory")
        .to_path_buf();

    // Construct the log file path relative to the executable directory
    let mut log_file_path = exe_dir;
    log_file_path.push("logs");
    log_file_path.push("launcher.log");

    // Create the logs directory if it doesn't exist
    std::fs::create_dir_all(log_file_path.parent().unwrap())
        .expect("Failed to create log directory");

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
    match setup_logging() {
        Ok(_) => log::info!("Logging successfully set up"),
        Err(e) => {
            log::error!("Failed to initialize logging: {:?}", e);
            eprintln!("Failed to initialize logging: {:?}", e); // Fallback to print to stderr
        }
    }
    // setup_logging().expect("failed to initialize logging");
    // panic::set_hook(Box::new(|panic_info| {
    //     if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
    //         error!("PANIC! at the disco: {}", s);
    //     } else {
    //         error!("PANIC! with some big boy panics");
    //     }
    // }));

    panic::set_hook(Box::new(|panic_info| {
        let payload = panic_info.payload();

        // Try to downcast to &str
        if let Some(s) = payload.downcast_ref::<&str>() {
            error!("PANIC! at the disco: {}", s);
        }
        // Try to downcast to String
        else if let Some(s) = payload.downcast_ref::<String>() {
            error!("PANIC! at the disco: {}", s);
        }
        // If it's not a &str or String, log the type and some additional info
        else {
            let type_name = payload.type_id();
            let location = panic_info
                .location()
                .unwrap_or_else(|| panic::Location::caller());
            error!(
                "PANIC! with some big boy panics. Type: {:?}, Location: {}:{}",
                type_name,
                location.file(),
                location.line()
            );
        }
    }));
    tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            download_files,
            get_patches,
            open_app,
            sha256_digest,
            log_message
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
