use crate::activity::types::ActivityEvent;
use crate::types::Result;
use chrono::Utc;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use uuid::Uuid;

#[must_use]
pub fn get_activity_log_path() -> PathBuf {
    super::paths::get_activity_log_path()
}

pub fn append_event(pane_id: Uuid, event: &ActivityEvent) -> Result<()> {
    let path = get_activity_log_path();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut persisted_event = event.clone();
    persisted_event.pane_id = pane_id;

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let line = serde_json::to_string(&persisted_event)?;
    writeln!(file, "{line}")?;

    Ok(())
}

pub fn load_all_events() -> Result<Vec<(Uuid, ActivityEvent)>> {
    let path = get_activity_log_path();

    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut events = Vec::new();

    for line_result in reader.lines() {
        let line = line_result?;
        if line.trim().is_empty() {
            continue;
        }

        let event: ActivityEvent = serde_json::from_str(&line)?;
        events.push((event.pane_id, event));
    }

    Ok(events)
}

pub fn prune_events(retention_days: i64) -> Result<()> {
    let path = get_activity_log_path();

    if !path.exists() {
        return Ok(());
    }

    let cutoff_time = Utc::now() - chrono::Duration::days(retention_days);
    let retained_events: Vec<ActivityEvent> = load_all_events()?
        .into_iter()
        .map(|(_, event)| event)
        .filter(|event| event.timestamp > cutoff_time)
        .collect();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let temporary_path = path.with_file_name("activity.jsonl.tmp");

    {
        let mut file = File::create(&temporary_path)?;
        for event in retained_events {
            let line = serde_json::to_string(&event)?;
            writeln!(file, "{line}")?;
        }
        file.sync_all()?;
    }

    fs::rename(temporary_path, path)?;

    Ok(())
}
