use serde::{
    de::{self, Visitor},
    ser::SerializeStruct,
    Deserialize, Serialize,
};
use std::fmt;

use crate::request::JsonRpcVersion;

#[derive(Debug)]
pub struct Notification {
    method: String,
    params: Option<serde_json::Value>,
}

impl Serialize for Notification {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer
            .serialize_struct("Notification", if self.params.is_some() { 3 } else { 2 })?;
        state.serialize_field("jsonrpc", "2.0")?;
        state.serialize_field("method", &self.method)?;
        if let Some(params) = &self.params {
            state.serialize_field("params", params)?;
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for Notification {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct NotificationVisitor;

        impl<'de> Visitor<'de> for NotificationVisitor {
            type Value = Notification;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a JSON-RPC 2.0 notification object")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut jsonrpc = None;
                let mut method = None;
                let mut params = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "jsonrpc" => {
                            if jsonrpc.is_some() {
                                return Err(de::Error::duplicate_field("jsonrpc"));
                            }
                            jsonrpc = Some(map.next_value::<JsonRpcVersion>()?);
                        }
                        "method" => {
                            if method.is_some() {
                                return Err(de::Error::duplicate_field("method"));
                            }
                            method = Some(map.next_value()?);
                        }
                        "params" => {
                            if params.is_some() {
                                return Err(de::Error::duplicate_field("params"));
                            }
                            params = Some(Some(map.next_value()?));
                        }
                        "id" => {
                            return Err(de::Error::custom("notifications must not include an id"))
                        }
                        _ => {
                            return Err(de::Error::unknown_field(
                                &key,
                                &["jsonrpc", "method", "params"],
                            ))
                        }
                    }
                }

                let _version = jsonrpc.ok_or_else(|| de::Error::missing_field("jsonrpc"))?;
                let method = method.ok_or_else(|| de::Error::missing_field("method"))?;
                let params = params.unwrap_or(None);

                Ok(Notification { method, params })
            }
        }

        deserializer.deserialize_map(NotificationVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serialize_notification_with_params() {
        let notification = Notification {
            method: "update".to_string(),
            params: Some(json!([1, 2, 3])),
        };

        assert_eq!(
            json!({
                "jsonrpc": "2.0",
                "method": "update",
                "params": [1, 2, 3]
            }),
            serde_json::to_value(notification).unwrap()
        );
    }

    #[test]
    fn serialize_notification_without_params() {
        let notification = Notification {
            method: "update".to_string(),
            params: None,
        };

        assert_eq!(
            json!({
                "jsonrpc": "2.0",
                "method": "update"
            }),
            serde_json::to_value(notification).unwrap()
        );
    }

    #[test]
    fn deserialize_valid_notification() {
        let json = json!({
            "jsonrpc": "2.0",
            "method": "update",
            "params": [1, 2, 3]
        });

        let notification: Notification = serde_json::from_value(json).unwrap();
        assert_eq!(notification.method, "update");
        assert_eq!(notification.params, Some(json!([1, 2, 3])));
    }

    #[test]
    fn deserialize_notification_without_params() {
        let json = json!({
            "jsonrpc": "2.0",
            "method": "update"
        });

        let notification: Notification = serde_json::from_value(json).unwrap();
        assert_eq!(notification.method, "update");
        assert_eq!(notification.params, None);
    }

    #[test]
    fn reject_notification_without_version() {
        let json = json!({
            "method": "update",
            "params": [1, 2, 3]
        });

        assert!(serde_json::from_value::<Notification>(json).is_err());
    }

    #[test]
    fn reject_notification_with_wrong_version() {
        let json = json!({
            "jsonrpc": "1.0",
            "method": "update",
            "params": [1, 2, 3]
        });

        assert!(serde_json::from_value::<Notification>(json).is_err());
    }

    #[test]
    fn reject_notification_without_method() {
        let json = json!({
            "jsonrpc": "2.0",
            "params": [1, 2, 3]
        });

        assert!(serde_json::from_value::<Notification>(json).is_err());
    }

    #[test]
    fn reject_notification_with_id() {
        let json = json!({
            "jsonrpc": "2.0",
            "method": "update",
            "params": [1, 2, 3],
            "id": 1
        });

        assert!(serde_json::from_value::<Notification>(json).is_err());
    }
}
