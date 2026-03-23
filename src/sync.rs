use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::mpsc;

const BATCH_SIZE: usize = 10;

#[derive(Debug, Clone)]
pub struct SyncProgress {
    pub current: usize,
    pub total: usize,
    pub phase: String,
}

#[derive(Debug, Clone, Default)]
pub struct SyncResult {
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SyncRequestFile {
    filename: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct SyncRequest {
    files: Vec<SyncRequestFile>,
}

#[derive(Debug, Deserialize)]
struct SyncResponse {
    #[serde(default)]
    imported: usize,
    #[serde(default)]
    skipped: usize,
    #[serde(default)]
    errors: Vec<String>,
}

pub fn run_sync(
    api_url: String,
    api_token: String,
    folder_path: String,
    progress_tx: mpsc::Sender<SyncProgress>,
) -> SyncResult {
    let dir = PathBuf::from(&folder_path);
    let run_files: Vec<PathBuf> = match std::fs::read_dir(&dir) {
        Ok(entries) => entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "run"))
            .collect(),
        Err(e) => {
            return SyncResult {
                errors: vec![format!("Failed to read directory: {e}")],
                ..Default::default()
            };
        }
    };

    let total = run_files.len();
    if total == 0 {
        let _ = progress_tx.send(SyncProgress {
            current: 0,
            total: 0,
            phase: "No .run files found".to_string(),
        });
        return SyncResult::default();
    }

    let sync_url = format!("{}/api/runs/sync", api_url.trim_end_matches('/'));
    let client = reqwest::blocking::Client::new();
    let mut result = SyncResult::default();
    let batches: Vec<&[PathBuf]> = run_files.chunks(BATCH_SIZE).collect();
    let mut files_processed = 0;

    for (batch_idx, batch) in batches.iter().enumerate() {
        let _ = progress_tx.send(SyncProgress {
            current: files_processed,
            total,
            phase: format!("Uploading batch {}/{}", batch_idx + 1, batches.len()),
        });

        let mut request_files = Vec::new();

        for file_path in *batch {
            match std::fs::read_to_string(file_path) {
                Ok(content) => {
                    let filename = file_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    request_files.push(SyncRequestFile { filename, content });
                }
                Err(e) => {
                    result
                        .errors
                        .push(format!("Failed to read {}: {e}", file_path.display()));
                }
            }
        }

        if request_files.is_empty() {
            files_processed += batch.len();
            continue;
        }

        let body = SyncRequest { files: request_files };

        match client
            .post(&sync_url)
            .header("Authorization", format!("Bearer {}", api_token))
            .json(&body)
            .send()
        {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    match response.json::<SyncResponse>() {
                        Ok(resp) => {
                            result.imported += resp.imported;
                            result.skipped += resp.skipped;
                            result.errors.extend(resp.errors);
                        }
                        Err(e) => {
                            result
                                .errors
                                .push(format!("Failed to parse response: {e}"));
                        }
                    }
                } else if status.as_u16() == 401 {
                    result
                        .errors
                        .push("Token rejected (401 Unauthorized)".to_string());
                    break;
                } else if status.as_u16() == 413 {
                    result
                        .errors
                        .push("Batch too large (413 Payload Too Large)".to_string());
                } else {
                    let body = response.text().unwrap_or_default();
                    result
                        .errors
                        .push(format!("Server error ({status}): {body}"));
                }
            }
            Err(e) => {
                result.errors.push(format!("Network error: {e}"));
                break;
            }
        }

        files_processed += batch.len();
    }

    let _ = progress_tx.send(SyncProgress {
        current: total,
        total,
        phase: "Done".to_string(),
    });

    result
}
