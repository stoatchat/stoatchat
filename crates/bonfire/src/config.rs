use async_tungstenite::tungstenite::{handshake, Message};
use futures::channel::oneshot::Sender;
use once_cell::sync::Lazy;
use regex::Regex;
use revolt_database::events::client::ReadyPayloadFields;
use revolt_result::{create_error, Result};
use serde::{Deserialize, Serialize};

/// matches either a single word ie "users" or a key and value ie "settings[notifications]"
static READY_PAYLOAD_FIELD_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"^(\w+)(?:\[(\S+)\])?$"#).unwrap());

/// Enumeration of supported protocol formats
#[derive(Debug)]
pub enum ProtocolFormat {
    Json,
    Msgpack,
}

/// User-provided protocol configuration
#[derive(Debug)]
pub struct ProtocolConfiguration {
    protocol_version: i32,
    format: ProtocolFormat,
    session_token: Option<String>,
    ready_payload_fields: ReadyPayloadFields,
}

impl ProtocolConfiguration {
    /// Create a new protocol configuration object from provided data
    pub fn from(
        protocol_version: i32,
        format: ProtocolFormat,
        session_token: Option<String>,
        ready_payload_fields: ReadyPayloadFields,
    ) -> Self {
        Self {
            protocol_version,
            format,
            session_token,
            ready_payload_fields,
        }
    }

    /// Decode some WebSocket message into a T: Deserialize using the client's specified protocol format
    pub fn decode<'a, T: Deserialize<'a>>(&self, msg: &'a Message) -> Result<T> {
        match self.format {
            ProtocolFormat::Json => {
                if let Message::Text(text) = msg {
                    serde_json::from_str(text).map_err(|_| create_error!(InternalError))
                } else {
                    Err(create_error!(InternalError))
                }
            }
            ProtocolFormat::Msgpack => {
                if let Message::Binary(buf) = msg {
                    rmp_serde::from_slice(buf).map_err(|_| create_error!(InternalError))
                } else {
                    Err(create_error!(InternalError))
                }
            }
        }
    }

    /// Encode T: Serialize into a WebSocket message using the client's specified protocol format
    pub fn encode<T: Serialize>(&self, data: &T) -> Message {
        match self.format {
            ProtocolFormat::Json => {
                Message::Text(serde_json::to_string(data).expect("Failed to serialise (as json)."))
            }
            ProtocolFormat::Msgpack => Message::Binary(
                rmp_serde::to_vec_named(data).expect("Failed to serialise (as msgpack)."),
            ),
        }
    }

    /// Set the current session token
    pub fn set_session_token(&mut self, token: String) {
        self.session_token.replace(token);
    }

    /// Get the current session token
    pub fn get_session_token(&self) -> &Option<String> {
        &self.session_token
    }

    /// Get the protocol version specified
    pub fn get_protocol_version(&self) -> i32 {
        self.protocol_version
    }

    /// Get the protocol format specified
    pub fn get_protocol_format(&self) -> &ProtocolFormat {
        &self.format
    }

    /// Get ready payload fields
    pub fn get_ready_payload_fields(&self) -> &ReadyPayloadFields {
        &self.ready_payload_fields
    }
}

/// Object holding one side of a channel for receiving the parsed information
pub struct WebsocketHandshakeCallback {
    sender: Sender<ProtocolConfiguration>,
}

impl WebsocketHandshakeCallback {
    /// Create a callback using a given sender
    pub fn from(sender: Sender<ProtocolConfiguration>) -> Self {
        Self { sender }
    }
}

impl handshake::server::Callback for WebsocketHandshakeCallback {
    /// Handle request to create a new WebSocket connection
    fn on_request(
        self,
        request: &handshake::server::Request,
        response: handshake::server::Response,
    ) -> Result<handshake::server::Response, handshake::server::ErrorResponse> {
        // Take and parse query parameters from the URI.
        let query = request.uri().query().unwrap_or_default();
        let params = querystring::querify(query);

        // Set default values for the protocol.
        let mut protocol_version = 1;
        let mut format = ProtocolFormat::Json;
        let mut session_token = None;
        let mut ready_payload_fields = if params.iter().any(|(k, _)| *k == "ready") {
            // If they pass the ready field, set all fields to false

            ReadyPayloadFields {
                users: false,
                servers: false,
                channels: false,
                members: false,
                emojis: false,
                voice_states: false,
                user_settings: Vec::new(),
                channel_unreads: false,
                policy_changes: false,
            }
        } else {
            ReadyPayloadFields::default()
        };

        // Parse and map parameters from key-value to known variables.
        for (key, value) in params {
            match key {
                "version" => {
                    if let Ok(version) = value.parse() {
                        protocol_version = version;
                    }
                }
                "format" => match value {
                    "json" => format = ProtocolFormat::Json,
                    "msgpack" => format = ProtocolFormat::Msgpack,
                    _ => {}
                },
                "token" => session_token = Some(value.into()),
                "ready" => {
                    // Re-enable all the fields the client specifies
                    if let Some(captures) = READY_PAYLOAD_FIELD_REGEX.captures(value) {
                        if let Some(field) = captures.get(0) {
                            match field.as_str() {
                                "users" => ready_payload_fields.users = true,
                                "servers" => ready_payload_fields.servers = true,
                                "channels" => ready_payload_fields.channels = true,
                                "members" => ready_payload_fields.members = true,
                                "emojis" => ready_payload_fields.emojis = true,
                                "voice_states" => ready_payload_fields.voice_states = true,
                                "channel_unreads" => ready_payload_fields.channel_unreads = true,
                                "user_settings" => {
                                    if let Some(subkey) = captures.get(1) {
                                        ready_payload_fields
                                            .user_settings
                                            .push(subkey.as_str().to_string());
                                    }
                                }
                                "policy_changes" => ready_payload_fields.policy_changes = true,
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Send configuration information back from this callback.
        // We have to use a channel as this function does not borrow mutably.
        if self
            .sender
            .send(ProtocolConfiguration {
                protocol_version,
                format,
                session_token,
                ready_payload_fields,
            })
            .is_ok()
        {
            Ok(response)
        } else {
            Err(handshake::server::ErrorResponse::new(None))
        }
    }
}
