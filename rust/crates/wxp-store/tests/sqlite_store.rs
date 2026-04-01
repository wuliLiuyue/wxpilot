use wxp_store::{RequestLog, StoreWriter};

#[test]
fn request_logs_are_truncated_at_2000_chars() {
    let writer = StoreWriter::memory().unwrap();
    let oversized = "x".repeat(2100);
    let log = RequestLog::new("GET", "https://wxpilot.test", Some(oversized.clone()), None);
    writer.insert_request(log).unwrap();
    let stored = writer.list_requests(20, true).unwrap();
    assert_eq!(stored[0].req_body.as_ref().unwrap().len(), 2001);
}
