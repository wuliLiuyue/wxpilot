use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use wxp_store::{RequestLog, StoreWriter};

fn unique_db_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();
    std::env::temp_dir().join(format!("wxp-store-{name}-{nanos}/requests.sqlite3"))
}

#[test]
fn request_logs_are_truncated_at_2000_chars() {
    let writer = StoreWriter::memory().unwrap();
    let oversized = "x".repeat(2100);
    let log = RequestLog::new("GET", "https://wxpilot.test", Some(oversized.clone()), None);
    writer.insert_request(log).unwrap();
    let stored = writer.list_requests(20, true).unwrap();
    assert_eq!(stored[0].req_body.as_ref().unwrap().len(), 2001);
}

#[test]
fn request_log_metadata_is_persisted() {
    let writer = StoreWriter::memory().unwrap();
    writer
        .insert_request(RequestLog {
            method: "POST".into(),
            url: "https://wxpilot.test/meta".into(),
            req_body: Some("{\"hello\":\"world\"}".into()),
            res_body: Some("{\"ok\":true}".into()),
            status: Some(201),
            ts: 1_775_196_000_123,
            mocked: true,
        })
        .unwrap();

    let stored = writer.list_requests(20, true).unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].status, Some(201));
    assert_eq!(stored[0].ts, 1_775_196_000_123);
    assert!(stored[0].mocked);
}

#[test]
fn requests_are_read_back_in_reverse_insert_order_and_can_be_cleared() {
    let writer = StoreWriter::memory().unwrap();
    writer
        .insert_request(RequestLog::new(
            "GET",
            "https://wxpilot.test/first",
            None,
            None,
        ))
        .unwrap();
    writer
        .insert_request(RequestLog::new(
            "POST",
            "https://wxpilot.test/second",
            None,
            None,
        ))
        .unwrap();

    let stored = writer.list_requests(20, false).unwrap();
    assert_eq!(stored.len(), 2);
    assert_eq!(stored[0].method, "POST");
    assert_eq!(stored[0].url, "https://wxpilot.test/second");
    assert_eq!(stored[1].method, "GET");

    writer.clear_requests().unwrap();
    let cleared = writer.list_requests(20, false).unwrap();
    assert!(cleared.is_empty());
}

#[test]
fn list_requests_omits_body_by_default() {
    let writer = StoreWriter::memory().unwrap();
    writer
        .insert_request(RequestLog::new(
            "GET",
            "https://wxpilot.test/body",
            Some("{\"hello\":\"world\"}".into()),
            Some("{\"ok\":true}".into()),
        ))
        .unwrap();

    let stored = writer.list_requests(20, false).unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].req_body, None);
    assert_eq!(stored[0].res_body, None);
}

#[test]
fn list_requests_accepts_i64_max_limit_without_overflowing() {
    let writer = StoreWriter::memory().unwrap();
    writer
        .insert_request(RequestLog::new(
            "GET",
            "https://wxpilot.test/overflow",
            None,
            None,
        ))
        .unwrap();

    let stored = writer.list_requests(i64::MAX as usize, false).unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].url, "https://wxpilot.test/overflow");
}

#[test]
fn file_store_persists_requests_across_reopen() {
    let db_path = unique_db_path("persist-reopen");
    let writer = StoreWriter::open(&db_path).unwrap();
    writer
        .insert_request(RequestLog::new(
            "POST",
            "https://wxpilot.test/persist",
            Some("{\"hello\":\"world\"}".into()),
            Some("{\"ok\":true}".into()),
        ))
        .unwrap();

    drop(writer);

    let reopened = StoreWriter::open(&db_path).unwrap();
    let stored = reopened.list_requests(20, true).unwrap();

    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].method, "POST");
    assert_eq!(stored[0].url, "https://wxpilot.test/persist");
    assert_eq!(stored[0].req_body.as_deref(), Some("{\"hello\":\"world\"}"));
    assert_eq!(stored[0].res_body.as_deref(), Some("{\"ok\":true}"));
}

#[cfg(target_pointer_width = "64")]
#[test]
fn list_requests_rejects_limits_larger_than_i64_max() {
    let writer = StoreWriter::memory().unwrap();

    let error = writer
        .list_requests(usize::MAX, false)
        .expect_err("limits above i64::MAX should fail");

    assert!(
        error.to_string().contains("request limit exceeds i64::MAX"),
        "unexpected error: {error}"
    );
}
