use serde::{
    de::{self, Visitor},
    ser::SerializeStruct,
    Deserialize, Serialize,
};
use serde_json::Number;
use std::fmt;

#[derive(Debug)]
pub struct Request {
    method: String,
    params: Option<serde_json::Value>,
    id: RequestId,
}

impl Serialize for Request {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state =
            serializer.serialize_struct("Request", if self.params.is_some() { 4 } else { 3 })?;
        // The JSON-RPC 2.0 spec requires that the "jsonrpc" field be present and set to "2.0".
        state.serialize_field("jsonrpc", "2.0")?;
        state.serialize_field("method", &self.method)?;
        if let Some(params) = &self.params {
            state.serialize_field("params", params)?;
        }
        state.serialize_field("id", &self.id)?;
        state.end()
    }
}

/// This is just a marker struct to ensure that the JSON-RPC version is "2.0".
/// This lets us consider it a deserialization error if it's not.
#[derive(Debug)]
pub(crate) struct JsonRpcVersion;

impl<'de> Deserialize<'de> for JsonRpcVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct VersionVisitor;

        impl Visitor<'_> for VersionVisitor {
            type Value = JsonRpcVersion;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a JSON-RPC 2.0 version string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value == "2.0" {
                    Ok(JsonRpcVersion)
                } else {
                    Err(E::custom(format!("invalid JSON-RPC version: {}", value)))
                }
            }
        }

        deserializer.deserialize_str(VersionVisitor)
    }
}

impl<'de> Deserialize<'de> for Request {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct RequestVisitor;

        impl<'de> Visitor<'de> for RequestVisitor {
            type Value = Request;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a JSON-RPC 2.0 request object")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut jsonrpc = None;
                let mut method = None;
                let mut params = None;
                let mut id = None;

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
                            if id.is_some() {
                                return Err(de::Error::duplicate_field("id"));
                            }
                            id = Some(map.next_value()?);
                        }
                        _ => {
                            return Err(de::Error::unknown_field(
                                &key,
                                &["jsonrpc", "method", "params", "id"],
                            ))
                        }
                    }
                }

                let _version = jsonrpc.ok_or_else(|| de::Error::missing_field("jsonrpc"))?;
                let method = method.ok_or_else(|| de::Error::missing_field("method"))?;
                let params = params.unwrap_or(None);
                let id = id.ok_or_else(|| de::Error::missing_field("id"))?;

                Ok(Request { method, params, id })
            }
        }

        deserializer.deserialize_map(RequestVisitor)
    }
}

/// Represents a JSON-RPC 2.0 request ID. The JSON-RPC 2.0 spec
/// defines that the request ID can be either a number, a string,
/// or null.
#[derive(Debug, Clone, PartialEq)]
pub enum RequestId {
    Number(Number),
    String(String),
    Null,
}

impl Serialize for RequestId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            RequestId::Number(n) => n.serialize(serializer),
            RequestId::String(s) => serializer.serialize_str(s),
            RequestId::Null => serializer.serialize_none(),
        }
    }
}

impl<'de> Deserialize<'de> for RequestId {
    fn deserialize<D>(deserializer: D) -> Result<RequestId, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct RequestIdVisitor;

        impl Visitor<'_> for RequestIdVisitor {
            type Value = RequestId;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string, number, or null")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(RequestId::String(value.to_owned()))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(RequestId::Number(value.into()))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(RequestId::Number(value.into()))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Number::from_f64(value)
                    .map(RequestId::Number)
                    .ok_or_else(|| de::Error::custom("invalid number"))
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(RequestId::Null)
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                // I'm not sure if this can actually be exercised.
                // The deserialize_null_id test passes without this.
                Ok(RequestId::Null)
            }
        }

        deserializer.deserialize_any(RequestIdVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_value, json};

    #[test]
    fn serialize_number_id() {
        assert_eq!(
            json!(42),
            serde_json::to_value(RequestId::Number(42.into())).unwrap()
        );
    }

    #[test]
    fn serialize_string_id() {
        assert_eq!(
            json!("abc"),
            serde_json::to_value(RequestId::String("abc".to_string())).unwrap()
        );
    }

    #[test]
    fn serialize_null_id() {
        assert_eq!(json!(null), serde_json::to_value(RequestId::Null).unwrap());
    }

    #[test]
    fn deserialize_number_id() {
        assert_eq!(
            RequestId::Number(42.into()),
            from_value::<RequestId>(json!(42)).unwrap()
        );
    }

    #[test]
    fn deserialize_string_id() {
        assert_eq!(
            RequestId::String("abc".to_string()),
            from_value::<RequestId>(json!("abc")).unwrap()
        );
    }

    #[test]
    fn deserialize_float_id() {
        assert_eq!(
            RequestId::Number(Number::from_f64(42.5).unwrap()),
            from_value::<RequestId>(json!(42.5)).unwrap()
        );
    }

    #[test]
    fn deserialize_null_id() {
        assert_eq!(
            RequestId::Null,
            from_value::<RequestId>(json!(null)).unwrap()
        );
    }

    #[test]
    fn reject_array() {
        assert!(from_value::<RequestId>(json!([1, 2, 3])).is_err());
    }

    #[test]
    fn reject_object() {
        assert!(from_value::<RequestId>(json!({"id": 1})).is_err());
    }

    #[test]
    fn reject_boolean() {
        assert!(from_value::<RequestId>(json!(true)).is_err());
    }

    #[test]
    fn deserialize_valid_request() {
        let json = json!({
            "jsonrpc": "2.0",
            "method": "subtract",
            "params": [42, 23],
            "id": 1
        });

        let request: Request = serde_json::from_value(json).unwrap();
        assert_eq!(request.method, "subtract");
        assert_eq!(request.params, Some(json!([42, 23])));
        assert_eq!(request.id, RequestId::Number(1.into()));
    }

    #[test]
    fn deserialize_request_without_params() {
        let json = json!({
            "jsonrpc": "2.0",
            "method": "ping",
            "id": 1
        });

        let request: Request = serde_json::from_value(json).unwrap();
        assert_eq!(request.method, "ping");
        assert_eq!(request.params, None);
        assert_eq!(request.id, RequestId::Number(1.into()));
    }

    #[test]
    fn reject_request_without_version() {
        let json = json!({
            "method": "subtract",
            "params": [42, 23],
            "id": 1
        });

        assert!(serde_json::from_value::<Request>(json).is_err());
    }

    #[test]
    fn reject_request_with_wrong_version() {
        let json = json!({
            "jsonrpc": "1.0",
            "method": "subtract",
            "params": [42, 23],
            "id": 1
        });

        assert!(serde_json::from_value::<Request>(json).is_err());
    }

    #[test]
    fn reject_request_without_method() {
        let json = json!({
            "jsonrpc": "2.0",
            "params": [42, 23],
            "id": 1
        });

        assert!(serde_json::from_value::<Request>(json).is_err());
    }

    #[test]
    fn reject_request_without_id() {
        let json = json!({
            "jsonrpc": "2.0",
            "method": "subtract",
            "params": [42, 23]
        });

        assert!(serde_json::from_value::<Request>(json).is_err());
    }

    #[test]
    fn serialize_request_with_params() {
        let request = Request {
            method: "subtract".to_string(),
            params: Some(json!([42, 23])),
            id: RequestId::Number(1.into()),
        };

        assert_eq!(
            json!({
                "jsonrpc": "2.0",
                "method": "subtract",
                "params": [42, 23],
                "id": 1
            }),
            serde_json::to_value(request).unwrap()
        );
    }

    #[test]
    fn serialize_request_without_params() {
        let request = Request {
            method: "ping".to_string(),
            params: None,
            id: RequestId::Number(1.into()),
        };

        assert_eq!(
            json!({
                "jsonrpc": "2.0",
                "method": "ping",
                "id": 1
            }),
            serde_json::to_value(request).unwrap()
        );
    }
}
