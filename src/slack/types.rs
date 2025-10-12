use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackUserProfile {
    pub real_name: Option<String>,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub status_text: Option<String>,
    pub status_emoji: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackUser {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub is_bot: bool,
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default)]
    pub deleted: bool,
    pub profile: Option<SlackUserProfile>,
}

impl SlackUser {
    // Helper methods for accessing profile fields
    pub fn real_name(&self) -> Option<&str> {
        self.profile.as_ref()?.real_name.as_deref()
    }

    pub fn display_name(&self) -> Option<&str> {
        self.profile.as_ref()?.display_name.as_deref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackChannel {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub is_channel: bool,
    #[serde(default)]
    pub is_private: bool,
    #[serde(default)]
    pub is_archived: bool,
    #[serde(default)]
    pub is_general: bool,
    #[serde(default)]
    pub is_im: bool,
    #[serde(default)]
    pub is_mpim: bool,
    #[serde(default)]
    pub is_member: bool,
    pub created: Option<i64>,
    pub creator: Option<String>,
    pub num_members: Option<i32>,
    pub topic: Option<ChannelTopic>,
    pub purpose: Option<ChannelPurpose>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelTopic {
    pub value: String,
    pub creator: String,
    pub last_set: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelPurpose {
    pub value: String,
    pub creator: String,
    pub last_set: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackMessage {
    pub ts: String,
    pub user: Option<String>,
    pub text: String,
    pub thread_ts: Option<String>,
    pub reply_count: Option<i32>,
    pub reply_users: Option<Vec<String>>,
    pub reply_users_count: Option<i32>,
    pub latest_reply: Option<String>,
    pub parent_user_id: Option<String>,
    pub reactions: Option<Vec<Reaction>>,
    pub subtype: Option<String>,
    pub edited: Option<EditedInfo>,
    pub blocks: Option<Vec<serde_json::Value>>,
    pub attachments: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditedInfo {
    pub user: String,
    pub ts: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reaction {
    pub name: String,
    pub users: Vec<String>,
    pub count: i32,
}
