use serde::Deserialize;
use worker::{SqlStorage, SqlStorageValue};

#[derive(Deserialize)]
struct CountRow {
    count: i64,
}

pub fn init_schema(sql: &SqlStorage) -> worker::Result<()> {
    sql.exec(
        "CREATE TABLE IF NOT EXISTS daily_cap (
            date TEXT PRIMARY KEY,
            count INTEGER NOT NULL DEFAULT 0
        )",
        None,
    )?;
    sql.exec(
        "CREATE TABLE IF NOT EXISTS qa_stats (
            hash TEXT PRIMARY KEY,
            hits INTEGER NOT NULL DEFAULT 0,
            last_seen INTEGER NOT NULL
        )",
        None,
    )?;
    Ok(())
}

pub fn check_and_increment_daily(sql: &SqlStorage, date: &str, cap: u32) -> worker::Result<bool> {
    let rows: Vec<CountRow> = sql
        .exec(
            "SELECT count FROM daily_cap WHERE date = ?",
            vec![SqlStorageValue::String(date.to_string())],
        )?
        .to_array()?;
    let current = rows.into_iter().next().map(|r| r.count.max(0)).unwrap_or(0) as u32;
    if current >= cap {
        return Ok(false);
    }
    sql.exec(
        "INSERT INTO daily_cap (date, count) VALUES (?, 1)
         ON CONFLICT(date) DO UPDATE SET count = count + 1",
        vec![SqlStorageValue::String(date.to_string())],
    )?;
    Ok(true)
}

pub fn record_hit(sql: &SqlStorage, hash: &str, now_ms: i64) -> worker::Result<()> {
    sql.exec(
        "INSERT INTO qa_stats (hash, hits, last_seen) VALUES (?, 1, ?)
         ON CONFLICT(hash) DO UPDATE SET hits = hits + 1, last_seen = excluded.last_seen",
        vec![
            SqlStorageValue::String(hash.to_string()),
            SqlStorageValue::Integer(now_ms),
        ],
    )?;
    Ok(())
}
