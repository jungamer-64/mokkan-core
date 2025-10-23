// src/domain/audit/cursor_tests.rs
#[cfg(test)]
mod tests {
    use crate::domain::audit::cursor::AuditLogCursor;
    use chrono::Utc;

    #[test]
    fn cursor_encode_decode_roundtrip() {
        let now = Utc::now();
        let id = 42i64;
        let c = AuditLogCursor::new(now, id);
        let token = c.encode();
        let decoded = AuditLogCursor::decode(&token).expect("decode should succeed");
        assert_eq!(decoded.id, id);
        assert_eq!(decoded.created_at.timestamp(), now.timestamp());
    }
}
