use once_cell::sync::Lazy;
use serde::{ser::Serializer, Serialize};
use std::collections::{BTreeMap};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ContextTagKey {
    key: &'static str,
    max_len: usize,
}

impl ContextTagKey {
    const fn new(key: &'static str, max_len: usize) -> Self {
        Self { key, max_len }
    }
}

impl Serialize for ContextTagKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.key)
    }
}

/// Application version. Information in the application context fields is always about the
/// application that is sending the telemetry.
pub(crate) const APPLICATION_VERSION: ContextTagKey =
    ContextTagKey::new("ai.application.ver", 1024);

/// Unique client device id. Computer name in most cases.
#[allow(dead_code)]
pub(crate) const DEVICE_ID: ContextTagKey = ContextTagKey::new("ai.device.id", 1024);

/// Device locale using <language>-<REGION> pattern, following RFC 5646. Example 'en-US'.
#[allow(dead_code)]
pub(crate) const DEVICE_LOCALE: ContextTagKey = ContextTagKey::new("ai.device.locale", 64);

/// Model of the device the end user of the application is using. Used for client scenarios. If
/// this field is empty then it is derived from the user agent.
#[allow(dead_code)]
pub(crate) const DEVICE_MODEL: ContextTagKey = ContextTagKey::new("ai.device.model", 256);

/// Client device OEM name taken from the browser.
#[allow(dead_code)]
pub(crate) const DEVICE_OEM_NAME: ContextTagKey = ContextTagKey::new("ai.device.oemName", 256);

/// Operating system name and version of the device the end user of the application is using. If
/// this field is empty then it is derived from the user agent. Example 'Windows 10 Pro
/// 10.0.10586.0'
#[allow(dead_code)]
pub(crate) const DEVICE_OS_VERSION: ContextTagKey = ContextTagKey::new("ai.device.osVersion", 256);

/// The type of the device the end user of the application is using. Used primarily to distinguish
/// JavaScript telemetry from server side telemetry. Examples: 'PC', 'Phone', 'Browser'. 'PC' is
/// the default value.
#[allow(dead_code)]
pub(crate) const DEVICE_TYPE: ContextTagKey = ContextTagKey::new("ai.device.type", 64);

/// The IP address of the client device. IPv4 and IPv6 are supported. Information in the location
/// context fields is always about the end user. When telemetry is sent from a service, the
/// location context is about the user that initiated the operation in the service.
#[allow(dead_code)]
pub(crate) const LOCATION_IP: ContextTagKey = ContextTagKey::new("ai.location.ip", 46);

/// The country of the client device. If any of Country, Province, or City is specified, those
/// values will be preferred over geolocation of the IP address field. Information in the location
/// context fields is always about the end user. When telemetry is sent from a service, the
/// location context is about the user that initiated the operation in the service.
#[allow(dead_code)]
pub(crate) const LOCATION_COUNTRY: ContextTagKey = ContextTagKey::new("ai.location.country", 256);

/// The province/state of the client device. If any of Country, Province, or City is specified,
/// those values will be preferred over geolocation of the IP address field. Information in the
/// location context fields is always about the end user. When telemetry is sent from a service,
/// the location context is about the user that initiated the operation in the service.
#[allow(dead_code)]
pub(crate) const LOCATION_PROVINCE: ContextTagKey = ContextTagKey::new("ai.location.province", 256);

/// The city of the client device. If any of Country, Province, or City is specified, those values
/// will be preferred over geolocation of the IP address field. Information in the location context
/// fields is always about the end user. When telemetry is sent from a service, the location
/// context is about the user that initiated the operation in the service.
#[allow(dead_code)]
pub(crate) const LOCATION_CITY: ContextTagKey = ContextTagKey::new("ai.location.city", 256);

/// A unique identifier for the operation instance. The operation.id is created by either a request
/// or a page view. All other telemetry sets this to the value for the containing request or page
/// view. Operation.id is used for finding all the telemetry items for a specific operation
/// instance.
pub(crate) const OPERATION_ID: ContextTagKey = ContextTagKey::new("ai.operation.id", 128);

/// The name (group) of the operation. The operation.name is created by either a request or a page
/// view. All other telemetry items set this to the value for the containing request or page view.
/// Operation.name is used for finding all the telemetry items for a group of operations (i.e. 'GET
/// Home/Index').
pub(crate) const OPERATION_NAME: ContextTagKey = ContextTagKey::new("ai.operation.name", 1024);

/// The unique identifier of the telemetry item's immediate parent.
pub(crate) const OPERATION_PARENT_ID: ContextTagKey =
    ContextTagKey::new("ai.operation.parentId", 128);

/// Name of synthetic source. Some telemetry from the application may represent a synthetic
/// traffic. It may be web crawler indexing the web site, site availability tests or traces from
/// diagnostic libraries like Application Insights SDK itself.
#[allow(dead_code)]
pub(crate) const OPERATION_SYNTHETIC_SOURCE: ContextTagKey =
    ContextTagKey::new("ai.operation.syntheticSource", 1024);

/// The correlation vector is a light weight vector clock which can be used to identify and order
/// related events across clients and services.
#[allow(dead_code)]
pub(crate) const OPERATION_CORRELATION_VECTOR: ContextTagKey =
    ContextTagKey::new("ai.operation.correlationVector", 64);

/// Session ID - the instance of the user's interaction with the app. Information in the session
/// context fields is always about the end user. When telemetry is sent from a service, the session
/// context is about the user that initiated the operation in the service.
#[allow(dead_code)]
pub(crate) const SESSION_ID: ContextTagKey = ContextTagKey::new("ai.session.id", 64);

/// Boolean value indicating whether the session identified by ai.session.id is first for the user
/// or not.
#[allow(dead_code)]
pub(crate) const SESSION_IS_FIRST: ContextTagKey = ContextTagKey::new("ai.session.isFirst", 5);

/// In multi-tenant applications this is the account ID or name which the user is acting with.
/// Examples may be subscription ID for Azure portal or blog name blogging platform.
#[allow(dead_code)]
pub(crate) const USER_ACCOUNT_ID: ContextTagKey = ContextTagKey::new("ai.user.accountId", 1024);

/// Anonymous user id. Represents the end user of the application. When telemetry is sent from a
/// service, the user context is about the user that initiated the operation in the service.
#[allow(dead_code)]
pub(crate) const USER_ID: ContextTagKey = ContextTagKey::new("ai.user.id", 128);

/// Authenticated user id. The opposite of ai.user.id, this represents the user with a friendly
/// name. Since it's PII information it is not collected by default by most SDKs.
pub(crate) const USER_AUTH_USER_ID: ContextTagKey = ContextTagKey::new("ai.user.authUserId", 1024);

/// Name of the role the application is a part of. Maps directly to the role name in azure.
pub(crate) const CLOUD_ROLE: ContextTagKey = ContextTagKey::new("ai.cloud.role", 256);

/// Name of the instance where the application is running. Computer name for on-premisis, instance
/// name for Azure.
pub(crate) const CLOUD_ROLE_INSTANCE: ContextTagKey =
    ContextTagKey::new("ai.cloud.roleInstance", 256);

/// SDK version. See
/// https://github.com/Microsoft/ApplicationInsights-Home/blob/master/SDK-AUTHORING.md#sdk-version-specification
/// for information.
pub(crate) const INTERNAL_SDK_VERSION: ContextTagKey =
    ContextTagKey::new("ai.internal.sdkVersion", 64);

/// Agent version. Used to indicate the version of StatusMonitor installed on the computer if it is
/// used for data collection.
#[allow(dead_code)]
pub(crate) const INTERNAL_AGENT_VERSION: ContextTagKey =
    ContextTagKey::new("ai.internal.agentVersion", 64);

/// This is the node name used for billing purposes. Use it to override the standard detection of
/// nodes.
#[allow(dead_code)]
pub(crate) const INTERNAL_NODE_NAME: ContextTagKey =
    ContextTagKey::new("ai.internal.nodeName", 256);

#[derive(Debug, Serialize)]
pub(crate) struct Tags(BTreeMap<ContextTagKey, String>);

impl Tags {
    pub(crate) fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub(crate) fn insert(&mut self, key: ContextTagKey, mut value: String) -> Option<String> {
        value.truncate(key.max_len);
        self.0.insert(key, value)
    }

    #[cfg(test)]
    pub(crate) fn get(&self, key: &ContextTagKey) -> Option<&String> {
        self.0.get(key)
    }
}

pub(crate) static TAG_KEY_LOOKUP: Lazy<BTreeMap<&'static str, ContextTagKey>> = Lazy::new(|| {
    vec![
        ("ai.application.ver", APPLICATION_VERSION),
        ("ai.device.id", DEVICE_ID),
        ("ai.device.locale", DEVICE_LOCALE),
        ("ai.device.model", DEVICE_MODEL),
        ("ai.device.oemName", DEVICE_OEM_NAME),
        ("ai.device.osVersion", DEVICE_OS_VERSION),
        ("ai.device.type", DEVICE_TYPE),
        ("ai.location.ip", LOCATION_IP),
        ("ai.location.country", LOCATION_COUNTRY),
        ("ai.location.province", LOCATION_PROVINCE),
        ("ai.location.city", LOCATION_CITY),
        ("ai.operation.id", OPERATION_ID),
        ("ai.operation.name", OPERATION_NAME),
        ("ai.operation.parentId", OPERATION_PARENT_ID),
        ("ai.operation.syntheticSource", OPERATION_SYNTHETIC_SOURCE),
        ("ai.operation.correlationVector", OPERATION_CORRELATION_VECTOR),
        ("ai.session.id", SESSION_ID),
        ("ai.session.isFirst", SESSION_IS_FIRST),
        ("ai.user.accountId", USER_ACCOUNT_ID),
        ("ai.user.id", USER_ID),
        ("ai.user.authUserId", USER_AUTH_USER_ID),
        ("ai.cloud.role", CLOUD_ROLE),
        ("ai.cloud.roleInstance", CLOUD_ROLE_INSTANCE),
        ("ai.internal.sdkVersion", INTERNAL_SDK_VERSION),
        ("ai.internal.agentVersion", INTERNAL_AGENT_VERSION),
        ("ai.internal.nodeName", INTERNAL_NODE_NAME)
    ].into_iter().collect()
});