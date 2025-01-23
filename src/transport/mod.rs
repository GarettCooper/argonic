use serde::{
    de::{self, IntoDeserializer},
    Deserialize, Serialize,
};

use crate::{notification::Notification, request::Request, response::Response};

/// All the different types of message defined by the JSON-RPC 2.0 specification.
/// Any individual message sent or received over a transport layer will be one of these types.
#[derive(Debug)]
pub enum Message {
    Request(Request),
    Response(Response),
    Notification(Notification),
    BatchRequest(Vec<Request>),
}

impl Serialize for Message {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Message::Request(req) => req.serialize(serializer),
            Message::Response(resp) => resp.serialize(serializer),
            Message::Notification(notif) => notif.serialize(serializer),
            Message::BatchRequest(reqs) => reqs.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Message {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // This is based on the implementation of untagged enum deserialization in the serde
        // derive implementation. serde uses a private enum called Content as its intermediate
        // representation. We use serde_json::Value instead which _might_ not be correct if we
        // were handling data types other than JSON, but since argonic is a JSON-RPC library, it
        // should be fine.
        let value = serde_json::Value::deserialize(deserializer)?;
        let deserializer = serde_json::Value::into_deserializer(value);

        if let Ok(value) = Notification::deserialize(&deserializer) {
            return Ok(Message::Notification(value));
        }
        if let Ok(value) = Request::deserialize(&deserializer) {
            return Ok(Message::Request(value));
        }
        if let Ok(value) = Response::deserialize(&deserializer) {
            return Ok(Message::Response(value));
        }
        if let Ok(value) = Vec::<Request>::deserialize(&deserializer) {
            return Ok(Message::BatchRequest(value));
        }

        Err(de::Error::custom(
            "data did not match any variant of Message",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn deserialize_request() {
        let json = json!({
            "jsonrpc": "2.0",
            "method": "subtract",
            "params": [42, 23],
            "id": 1
        });

        match serde_json::from_value::<Message>(json).unwrap() {
            Message::Request(_) => {}
            _ => panic!("expected Request variant"),
        }
    }

    #[test]
    fn deserialize_notification() {
        let json = json!({
            "jsonrpc": "2.0",
            "method": "update",
            "params": [1, 2, 3]
        });

        match serde_json::from_value::<Message>(json).unwrap() {
            Message::Notification(_) => {}
            _ => panic!("expected Notification variant"),
        }
    }

    #[test]
    fn deserialize_response() {
        let json = json!({
            "jsonrpc": "2.0",
            "result": 19,
            "id": 1
        });

        match serde_json::from_value::<Message>(json).unwrap() {
            Message::Response(_) => {}
            _ => panic!("expected Response variant"),
        }
    }

    #[test]
    fn deserialize_batch_request() {
        let json = json!([
            {
                "jsonrpc": "2.0",
                "method": "subtract",
                "params": [42, 23],
                "id": 1
            },
            {
                "jsonrpc": "2.0",
                "method": "subtract",
                "params": [42, 23],
                "id": 2
            }
        ]);

        match serde_json::from_value::<Message>(json).unwrap() {
            Message::BatchRequest(requests) => assert_eq!(requests.len(), 2),
            _ => panic!("expected BatchRequest variant"),
        }
    }

    #[test]
    fn serialize_request() {
        let json = json!({
            "jsonrpc": "2.0",
            "method": "subtract",
            "params": [42, 23],
            "id": 1
        });

        let request: Request = serde_json::from_value(json.clone()).unwrap();
        let message = Message::Request(request);
        assert_eq!(json, serde_json::to_value(message).unwrap());
    }

    #[test]
    fn serialize_notification() {
        let json = json!({
            "jsonrpc": "2.0",
            "method": "update",
            "params": [1, 2, 3]
        });

        let notification: Notification = serde_json::from_value(json.clone()).unwrap();
        let message = Message::Notification(notification);
        assert_eq!(json, serde_json::to_value(message).unwrap());
    }

    #[test]
    fn serialize_response() {
        let json = json!({
            "jsonrpc": "2.0",
            "result": 19,
            "id": 1
        });

        let response: Response = serde_json::from_value(json.clone()).unwrap();
        let message = Message::Response(response);
        assert_eq!(json, serde_json::to_value(message).unwrap());
    }

    #[test]
    fn serialize_batch_request() {
        let json = json!([
            {
                "jsonrpc": "2.0",
                "method": "subtract",
                "params": [42, 23],
                "id": 1
            },
            {
                "jsonrpc": "2.0",
                "method": "subtract",
                "params": [42, 23],
                "id": 2
            }
        ]);

        let requests: Vec<Request> = serde_json::from_value(json.clone()).unwrap();
        let message = Message::BatchRequest(requests);
        assert_eq!(json, serde_json::to_value(message).unwrap());
    }
}
