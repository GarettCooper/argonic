use serde::{
    de::{self, Visitor},
    ser::SerializeStruct,
    Deserialize, Serialize,
};
use serde_json::Value;
use std::fmt;

use crate::request::{JsonRpcVersion, RequestId};

#[derive(Debug)]
pub struct Response {
    id: RequestId,
    result: ResponseResult,
}

impl Serialize for Response {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Response", 3)?;
        // The JSON-RPC 2.0 spec requires that the "jsonrpc" field be present and set to "2.0".
        state.serialize_field("jsonrpc", "2.0")?;
        state.serialize_field("id", &self.id)?;
        match &self.result {
            ResponseResult::Ok(result) => state.serialize_field("result", result)?,
            ResponseResult::Err(err) => state.serialize_field("error", err)?,
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for Response {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ResponseVisitor;

        impl<'de> Visitor<'de> for ResponseVisitor {
            type Value = Response;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a JSON-RPC 2.0 response object")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut jsonrpc = None;
                let mut id = None;
                let mut result = None;
                let mut error = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "jsonrpc" => {
                            if jsonrpc.is_some() {
                                return Err(de::Error::duplicate_field("jsonrpc"));
                            }
                            jsonrpc = Some(map.next_value::<JsonRpcVersion>()?);
                        }
                        "id" => {
                            if id.is_some() {
                                return Err(de::Error::duplicate_field("id"));
                            }
                            id = Some(map.next_value()?);
                        }
                        "result" => {
                            if result.is_some() {
                                return Err(de::Error::duplicate_field("result"));
                            }
                            if error.is_some() {
                                return Err(de::Error::custom(
                                    "cannot have both result and error fields",
                                ));
                            }
                            result = Some(ResponseResult::Ok(map.next_value()?));
                        }
                        "error" => {
                            if error.is_some() {
                                return Err(de::Error::duplicate_field("error"));
                            }
                            if result.is_some() {
                                return Err(de::Error::custom(
                                    "cannot have both result and error fields",
                                ));
                            }
                            error = Some(ResponseResult::Err(map.next_value()?));
                        }
                        _ => {
                            return Err(de::Error::unknown_field(
                                &key,
                                &["jsonrpc", "id", "result", "error"],
                            ))
                        }
                    }
                }

                let _version = jsonrpc.ok_or_else(|| de::Error::missing_field("jsonrpc"))?;
                let id = id.ok_or_else(|| de::Error::missing_field("id"))?;
                let result = result
                    .or(error)
                    .ok_or_else(|| de::Error::custom("missing both result and error fields"))?;

                Ok(Response { id, result })
            }
        }

        deserializer.deserialize_map(ResponseVisitor)
    }
}

#[derive(Debug)]
pub enum ResponseResult {
    Ok(Value),
    Err(ResponseError),
}

#[derive(Debug)]
pub struct ResponseError {
    code: ErrorCode,
    message: String,
    data: Option<Value>,
}

impl Serialize for ResponseError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("ResponseError", 3)?;
        state.serialize_field("code", &self.code)?;
        state.serialize_field("message", &self.message)?;
        state.serialize_field("data", &self.data)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for ResponseError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ResponseErrorVisitor;

        impl<'de> Visitor<'de> for ResponseErrorVisitor {
            type Value = ResponseError;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a JSON-RPC 2.0 error object")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut code = None;
                let mut message = None;
                let mut data: Option<Value> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "code" => {
                            if code.is_some() {
                                return Err(de::Error::duplicate_field("code"));
                            }
                            code = Some(map.next_value()?);
                        }
                        "message" => {
                            if message.is_some() {
                                return Err(de::Error::duplicate_field("message"));
                            }
                            message = Some(map.next_value()?);
                        }
                        "data" => {
                            if data.is_some() {
                                return Err(de::Error::duplicate_field("data"));
                            }
                            data = Some(map.next_value()?);
                        }
                        _ => {
                            return Err(de::Error::unknown_field(
                                &key,
                                &["code", "message", "data"],
                            ))
                        }
                    }
                }

                let code = code.ok_or_else(|| de::Error::missing_field("code"))?;
                let message = message.ok_or_else(|| de::Error::missing_field("message"))?;
                // We need to convert Some(null) to None.
                let data = data.and_then(|data| if data.is_null() { None } else { Some(data) });

                Ok(ResponseError {
                    code,
                    message,
                    data,
                })
            }
        }

        deserializer.deserialize_map(ResponseErrorVisitor)
    }
}

/// JSON-RPC 2.0 error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// Invalid JSON was received by the server.
    /// An error occurred on the server while parsing the JSON text.
    ParseError,
    /// The JSON sent is not a valid Request object.
    InvalidRequest,
    /// The method does not exist / is not available.
    MethodNotFound,
    /// Invalid method parameter(s).
    InvalidParams,
    /// Internal JSON-RPC error.
    InternalError,
    /// An implementation-defined server-error.
    ServerError(i64),
    /// Custom error code defined by the application.
    ApplicationError(i64),
}

impl From<ErrorCode> for i64 {
    fn from(code: ErrorCode) -> i64 {
        match code {
            ErrorCode::ParseError => -32700,
            ErrorCode::InvalidRequest => -32600,
            ErrorCode::MethodNotFound => -32601,
            ErrorCode::InvalidParams => -32602,
            ErrorCode::InternalError => -32603,
            ErrorCode::ServerError(code) => code,
            ErrorCode::ApplicationError(code) => code,
        }
    }
}

impl From<i64> for ErrorCode {
    fn from(code: i64) -> ErrorCode {
        match code {
            -32700 => ErrorCode::ParseError,
            -32600 => ErrorCode::InvalidRequest,
            -32601 => ErrorCode::MethodNotFound,
            -32602 => ErrorCode::InvalidParams,
            -32603 => ErrorCode::InternalError,
            code if code < -32000 && code > -32099 => ErrorCode::ServerError(code),
            code => ErrorCode::ApplicationError(code),
        }
    }
}

impl Serialize for ErrorCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        i64::from(*self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ErrorCode {
    fn deserialize<D>(deserializer: D) -> Result<ErrorCode, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let code = i64::deserialize(deserializer)?;
        Ok(ErrorCode::from(code))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_ok_serialization() {
        assert_eq!(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": {}
            }),
            serde_json::to_value(super::Response {
                id: RequestId::Number(1.into()),
                result: ResponseResult::Ok(json!({})),
            })
            .unwrap()
        )
    }

    #[test]
    fn test_err_serialization() {
        assert_eq!(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "error": {
                    "code": -32601,
                    "message": "Method not found",
                    "data": null
                }
            }),
            serde_json::to_value(super::Response {
                id: RequestId::Number(1.into()),
                result: ResponseResult::Err(ResponseError {
                    code: ErrorCode::MethodNotFound,
                    message: "Method not found".to_string(),
                    data: None,
                }),
            })
            .unwrap()
        )
    }

    #[test]
    fn test_error_deserialization() {
        let json = json!({
            "code": -32601,
            "message": "Method not found",
            "data": null
        });

        let error: ResponseError = serde_json::from_value(json).unwrap();
        assert_eq!(error.code, ErrorCode::MethodNotFound);
        assert_eq!(error.message, "Method not found");
        assert_eq!(error.data, None);
    }

    #[test]
    fn test_error_deserialization_with_data() {
        let json = json!({
            "code": -32602,
            "message": "Invalid params",
            "data": {"missing": "id"}
        });

        let error: ResponseError = serde_json::from_value(json).unwrap();
        assert_eq!(error.code, ErrorCode::InvalidParams);
        assert_eq!(error.message, "Invalid params");
        assert_eq!(error.data, Some(json!({"missing": "id"})));
    }

    #[test]
    fn reject_error_without_code() {
        let json = json!({
            "message": "Method not found",
            "data": null
        });

        assert!(serde_json::from_value::<ResponseError>(json).is_err());
    }

    #[test]
    fn reject_error_without_message() {
        let json = json!({
            "code": -32601,
            "data": null
        });

        assert!(serde_json::from_value::<ResponseError>(json).is_err());
    }

    #[test]
    fn test_ok_response_deserialization() {
        let json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {"status": "success"}
        });

        let response: Response = serde_json::from_value(json).unwrap();
        assert_eq!(response.id, RequestId::Number(1.into()));
        match response.result {
            ResponseResult::Ok(value) => assert_eq!(value, json!({"status": "success"})),
            ResponseResult::Err(_) => panic!("expected Ok result"),
        }
    }

    #[test]
    fn test_error_response_deserialization() {
        let json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -32601,
                "message": "Method not found",
                "data": null
            }
        });

        let response: Response = serde_json::from_value(json).unwrap();
        assert_eq!(response.id, RequestId::Number(1.into()));
        match response.result {
            ResponseResult::Err(error) => {
                assert_eq!(error.code, ErrorCode::MethodNotFound);
                assert_eq!(error.message, "Method not found");
                assert_eq!(error.data, None);
            }
            ResponseResult::Ok(_) => panic!("expected Err result"),
        }
    }

    #[test]
    fn reject_response_without_version() {
        let json = json!({
            "id": 1,
            "result": {}
        });

        assert!(serde_json::from_value::<Response>(json).is_err());
    }

    #[test]
    fn reject_response_with_wrong_version() {
        let json = json!({
            "jsonrpc": "1.0",
            "id": 1,
            "result": {}
        });

        assert!(serde_json::from_value::<Response>(json).is_err());
    }

    #[test]
    fn reject_response_without_id() {
        let json = json!({
            "jsonrpc": "2.0",
            "result": {}
        });

        assert!(serde_json::from_value::<Response>(json).is_err());
    }

    #[test]
    fn reject_response_with_both_result_and_error() {
        let json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {},
            "error": {
                "code": -32601,
                "message": "Method not found",
                "data": null
            }
        });

        assert!(serde_json::from_value::<Response>(json).is_err());
    }

    #[test]
    fn reject_response_without_result_or_error() {
        let json = json!({
            "jsonrpc": "2.0",
            "id": 1
        });

        assert!(serde_json::from_value::<Response>(json).is_err());
    }
}
