use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct Alert {
    pub id: i64,
    pub alert_data: Value,
    pub initial_response: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: i64,
    pub alert_id: i64,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

impl Alert {
    #[allow(dead_code)]
    pub fn from_row(
        id: i64,
        alert_data: String,
        initial_response: String,
        created_at: String,
        updated_at: String,
    ) -> Result<Self, serde_json::Error> {
        let alert_data: Value = serde_json::from_str(&alert_data)?;
        Ok(Alert {
            id,
            alert_data,
            initial_response,
            created_at,
            updated_at,
        })
    }
}

impl ChatMessage {
    pub fn from_row(
        id: i64,
        alert_id: i64,
        role: String,
        content: String,
        created_at: String,
    ) -> Self {
        ChatMessage {
            id,
            alert_id,
            role,
            content,
            created_at,
        }
    }
}

pub fn get_current_timestamp() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_from_row() {
        let alert_data = r#"{"message":"test","description":"test desc","details":{"prefix":"192.0.2.0/24","asn":"1234"}}"#;
        let initial_response = "Test response";
        let created_at = "2025-01-15T10:30:00Z";
        let updated_at = "2025-01-15T10:30:00Z";

        let alert = Alert::from_row(
            1,
            alert_data.to_string(),
            initial_response.to_string(),
            created_at.to_string(),
            updated_at.to_string(),
        )
        .unwrap();

        assert_eq!(alert.id, 1);
        assert_eq!(alert.initial_response, initial_response);
        assert_eq!(alert.created_at, created_at);
        assert_eq!(alert.updated_at, updated_at);
        assert!(alert.alert_data.is_object());
    }

    #[test]
    fn test_alert_from_row_invalid_json() {
        let result = Alert::from_row(
            1,
            "invalid json".to_string(),
            "response".to_string(),
            "2025-01-15T10:30:00Z".to_string(),
            "2025-01-15T10:30:00Z".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_chat_message_from_row() {
        let msg = ChatMessage::from_row(
            1,
            10,
            "user".to_string(),
            "Hello".to_string(),
            "2025-01-15T10:30:00Z".to_string(),
        );

        assert_eq!(msg.id, 1);
        assert_eq!(msg.alert_id, 10);
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");
        assert_eq!(msg.created_at, "2025-01-15T10:30:00Z");
    }

    #[test]
    fn test_get_current_timestamp() {
        let timestamp = get_current_timestamp();
        assert!(!timestamp.is_empty());
        // Should be valid RFC3339 format
        assert!(timestamp.contains('T'));
        assert!(timestamp.contains('Z') || timestamp.contains('+') || timestamp.contains('-'));
    }
}
