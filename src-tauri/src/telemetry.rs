use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use crate::config::app_data_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchRecord {
    pub timestamp: String,
    pub group_id: String,
    pub group_name: String,
    pub trigger: String,
    pub display_apply_ms: u64,
    pub post_action_ms: u64,
    pub total_ms: u64,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryStats {
    pub count: usize,
    pub median_display_apply_ms: u64,
    pub p95_display_apply_ms: u64,
    pub success_rate: f64,
}

fn telemetry_path() -> PathBuf {
    app_data_dir().join("telemetry.jsonl")
}

pub fn append_record(record: &SwitchRecord, retention: usize) -> Result<(), String> {
    let dir = app_data_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let mut records = read_all()?;
    records.push(record.clone());
    if records.len() > retention {
        let drain = records.len() - retention;
        records.drain(0..drain);
    }

    let path = telemetry_path();
    let mut file = fs::File::create(&path).map_err(|e| e.to_string())?;
    for r in &records {
        let line = serde_json::to_string(r).map_err(|e| e.to_string())?;
        writeln!(file, "{line}").map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn read_all() -> Result<Vec<SwitchRecord>, String> {
    let path = telemetry_path();
    if !path.exists() {
        return Ok(vec![]);
    }

    let file = fs::File::open(&path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();
    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(record) = serde_json::from_str::<SwitchRecord>(&line) {
            records.push(record);
        }
    }
    Ok(records)
}

pub fn recent_records(limit: usize) -> Result<Vec<SwitchRecord>, String> {
    let mut records = read_all()?;
    if records.len() > limit {
        records = records.split_off(records.len() - limit);
    }
    records.reverse();
    Ok(records)
}

pub fn compute_stats() -> Result<TelemetryStats, String> {
    let records = read_all()?;
    if records.is_empty() {
        return Ok(TelemetryStats {
            count: 0,
            median_display_apply_ms: 0,
            p95_display_apply_ms: 0,
            success_rate: 0.0,
        });
    }

    let mut times: Vec<u64> = records.iter().map(|r| r.display_apply_ms).collect();
    times.sort_unstable();
    let median = times[times.len() / 2];
    let p95_idx = ((times.len() as f64) * 0.95).ceil() as usize - 1;
    let p95 = times[p95_idx.min(times.len() - 1)];
    let successes = records.iter().filter(|r| r.success).count();

    Ok(TelemetryStats {
        count: records.len(),
        median_display_apply_ms: median,
        p95_display_apply_ms: p95,
        success_rate: successes as f64 / records.len() as f64,
    })
}

pub fn clear() -> Result<(), String> {
    let path = telemetry_path();
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn export_to(path: &str) -> Result<(), String> {
    let records = read_all()?;
    let content = records
        .iter()
        .map(|r| serde_json::to_string(r).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(path, content).map_err(|e| e.to_string())
}

pub fn make_record(
    group_id: &str,
    group_name: &str,
    trigger: &str,
    display_apply_ms: u64,
    post_action_ms: u64,
    total_ms: u64,
    success: bool,
    error: Option<String>,
) -> SwitchRecord {
    SwitchRecord {
        timestamp: Utc::now().to_rfc3339(),
        group_id: group_id.to_string(),
        group_name: group_name.to_string(),
        trigger: trigger.to_string(),
        display_apply_ms,
        post_action_ms,
        total_ms,
        success,
        error,
    }
}