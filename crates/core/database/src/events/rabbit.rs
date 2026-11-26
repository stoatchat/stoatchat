use std::collections::HashMap;

use revolt_models::v0::PushNotification;
use serde::{Deserialize, Serialize};

use crate::{SystemNotice, User};

#[derive(Serialize, Deserialize)]
pub struct MessageSentPayload {
    pub notification: PushNotification,
    pub users: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct MassMessageSentPayload {
    pub notifications: Vec<PushNotification>,
    pub server_id: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FRAcceptedPayload {
    pub accepted_user: User,
    pub user: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FRReceivedPayload {
    pub from_user: User,
    pub user: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GenericPayload {
    pub title: String,
    pub body: String,
    pub icon: Option<String>,
    pub user: User,
}

/// The system DM a notice was posted in, so clients can navigate to it
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SystemNoticeMessage {
    pub channel_id: String,
    pub message_id: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SystemNoticePayload {
    pub notice: SystemNotice,
    /// Absent if the system DM couldn't be sent
    pub message: Option<SystemNoticeMessage>,
    pub user: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DmCallPayload {
    pub initiator_id: String,
    pub channel_id: String,
    pub started_at: Option<String>,
    pub ended: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct InternalDmCallPayload {
    pub payload: DmCallPayload,
    pub recipients: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
#[allow(clippy::large_enum_variant)]
pub enum PayloadKind {
    MessageNotification(PushNotification),
    FRAccepted(FRAcceptedPayload),
    FRReceived(FRReceivedPayload),
    BadgeUpdate(usize),
    Generic(GenericPayload),
    DmCallStartEnd(DmCallPayload),
    SystemNotice(SystemNoticePayload),
}

#[derive(Serialize, Deserialize)]
pub struct PayloadToService {
    pub notification: PayloadKind,
    pub user_id: String,
    pub session_id: String,
    pub token: String,
    pub extras: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub struct AckPayload {
    pub user_id: String,
    pub channel_id: String,
    pub message_id: String,
}

/// This is not the same as the AckPayload above, as the state for this event is stored in redis to allow for state updates while the event is queued.
#[derive(Serialize, Deserialize, Debug)]
pub struct AckEventPayload {
    pub user_id: String,
    pub channel_id: Option<String>,
    pub server_id: Option<String>,
}
