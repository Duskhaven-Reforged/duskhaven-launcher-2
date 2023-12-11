// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// src-tauri/src/main.rs
use reqwest::Client;
use serde::Serialize;
use std::panic;
use std::time::Instant;
use tauri::Manager;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};

#[derive(Serialize)]
pub struct Progress {
    pub download_id: i64,
    pub filesize: u64,
    pub transfered: u64,
    pub transfer_rate: f64,
    pub percentage: f64,
}

#[tauri::command]
async fn download_file(
    app: tauri::AppHandle,
    url: String,
    destination: String,
) -> Result<(), String> {
    let client = Client::new();
    let total_size = client
        .head(&url)
        .send()
        .await
        .map_err(|err| err.to_string())?;

    if total_size.status().is_success() {
        let size = total_size
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|ct_len| ct_len.to_str().ok().and_then(|ct_len| ct_len.parse().ok()))
            .unwrap_or(0);

        let request = client.get(&url);
        let mut response = request.send().await.map_err(|err| err.to_string())?;

        let mut out = BufWriter::new(
            File::create(&destination)
                .await
                .map_err(|err| err.to_string())?,
        );
        let mut downloaded: u64 = 0;
        let start = Instant::now();
        let mut progress = Progress {
            download_id: 1, // example download_id
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
            //app.emit_all("DOWNLOAD_PROGRESS", &progress).unwrap();
        }
        match app.emit_all("DOWNLOAD_FINISHED", &progress) {
            Ok(_) => {
                println!("we done :{}%", progress.percentage.to_string());
                Ok(())
            }
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
        println!(
            "the problem is :{}%",
            total_size.status().as_str().to_string()
        );
        Err(total_size.status().as_str().to_string())
    }
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
        .invoke_handler(tauri::generate_handler![download_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
