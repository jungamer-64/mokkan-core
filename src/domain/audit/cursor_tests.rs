// src/domain/audit/cursor_tests.rs
#[cfg(test)]
mod tests {
    use crate::domain::audit::cursor::Cursor;
    use chrono::Utc;

    #[test]
    fn cursor_encode_decode_roundtrip() {
        let now = Utc::now();
        let id = 42i64;
        let c = Cursor::new(now, id);
        let token = c.encode();
        let decoded = Cursor::decode(&token).expect("decode should succeed");
        assert_eq!(decoded.id, id);
        assert_eq!(decoded.created_at.timestamp(), now.timestamp());
    }
}
