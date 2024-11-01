use derive_more::derive::{Deref, Display, From, Into};
use ipnet::IpNet;
use poem_openapi::Enum;
use serde::{Deserialize, Serialize};

use super::SipAuth;

#[derive(Debug, Display, Clone, From, Into, Deref, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppId(String);

#[derive(Debug, Clone, Deserialize)]
pub struct AppInfo {
    pub app_id: String,
    pub app_secret: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PhoneNumber {
    pub number: String,
    pub subnets: Vec<IpNet>,
    pub auth: Option<SipAuth>,
    pub app_id: String,
    pub hook: String,
    pub hook_content_type: HookContentType,
}

#[derive(Debug, Enum, Clone, Copy, Deserialize)]
pub enum HookContentType {
    Json,
    Protobuf,
}

#[derive(Debug, Deserialize)]
pub struct PhoneNumbersSyncResponse {
    pub numbers: Vec<PhoneNumber>,
}

#[derive(Debug, Deserialize)]
pub struct AppsSyncResponse {
    pub apps: Vec<AppInfo>,
}
