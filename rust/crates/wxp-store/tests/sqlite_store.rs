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
