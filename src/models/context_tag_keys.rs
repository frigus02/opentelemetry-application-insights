use serde::ser::{Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ContextTagKey(&'static str);

impl ContextTagKey {
    const fn new(key: &'static str) -> Self {
        ContextTagKey(key)
    }
}

impl Serialize for ContextTagKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.0)
    }
}

/// Application version. Information in the application context fields is always about the
/// application that is sending the telemetry.
#[allow(dead_code)]
pub(crate) const APPLICATION_VERSION: ContextTagKey = ContextTagKey::new("ai.application.ver");

/// Unique client device id. Computer name in most cases.
#[allow(dead_code)]
pub(crate) const DEVICE_ID: ContextTagKey = ContextTagKey::new("ai.device.id");

/// Device locale using <language>-<REGION> pattern, following RFC 5646. Example 'en-US'.
#[allow(dead_code)]
pub(crate) const DEVICE_LOCALE: ContextTagKey = ContextTagKey::new("ai.device.locale");

/// Model of the device the end user of the application is using. Used for client scenarios. If
/// this field is empty then it is derived from the user agent.
#[allow(dead_code)]
pub(crate) const DEVICE_MODEL: ContextTagKey = ContextTagKey::new("ai.device.model");

/// Client device OEM name taken from the browser.
#[allow(dead_code)]
pub(crate) const DEVICE_OEM_NAME: ContextTagKey = ContextTagKey::new("ai.device.oemName");

/// Operating system name and version of the device the end user of the application is using. If
/// this field is empty then it is derived from the user agent. Example 'Windows 10 Pro
/// 10.0.10586.0'
#[allow(dead_code)]
pub(crate) const DEVICE_OS_VERSION: ContextTagKey = ContextTagKey::new("ai.device.osVersion");

/// The type of the device the end user of the application is using. Used primarily to distinguish
/// JavaScript telemetry from server side telemetry. Examples: 'PC', 'Phone', 'Browser'. 'PC' is
/// the default value.
#[allow(dead_code)]
pub(crate) const DEVICE_TYPE: ContextTagKey = ContextTagKey::new("ai.device.type");

/// The IP address of the client device. IPv4 and IPv6 are supported. Information in the location
/// context fields is always about the end user. When telemetry is sent from a service, the
/// location context is about the user that initiated the operation in the service.
#[allow(dead_code)]
pub(crate) const LOCATION_IP: ContextTagKey = ContextTagKey::new("ai.location.ip");

/// The country of the client device. If any of Country, Province, or City is specified, those
/// values will be preferred over geolocation of the IP address field. Information in the location
/// context fields is always about the end user. When telemetry is sent from a service, the
/// location context is about the user that initiated the operation in the service.
#[allow(dead_code)]
pub(crate) const LOCATION_COUNTRY: ContextTagKey = ContextTagKey::new("ai.location.country");

/// The province/state of the client device. If any of Country, Province, or City is specified,
/// those values will be preferred over geolocation of the IP address field. Information in the
/// location context fields is always about the end user. When telemetry is sent from a service,
/// the location context is about the user that initiated the operation in the service.
#[allow(dead_code)]
pub(crate) const LOCATION_PROVINCE: ContextTagKey = ContextTagKey::new("ai.location.province");

/// The city of the client device. If any of Country, Province, or City is specified, those values
/// will be preferred over geolocation of the IP address field. Information in the location context
/// fields is always about the end user. When telemetry is sent from a service, the location
/// context is about the user that initiated the operation in the service.
#[allow(dead_code)]
pub(crate) const LOCATION_CITY: ContextTagKey = ContextTagKey::new("ai.location.city");

/// A unique identifier for the operation instance. The operation.id is created by either a request
/// or a page view. All other telemetry sets this to the value for the containing request or page
/// view. Operation.id is used for finding all the telemetry items for a specific operation
/// instance.
#[allow(dead_code)]
pub(crate) const OPERATION_ID: ContextTagKey = ContextTagKey::new("ai.operation.id");

/// The name (group) of the operation. The operation.name is created by either a request or a page
/// view. All other telemetry items set this to the value for the containing request or page view.
/// Operation.name is used for finding all the telemetry items for a group of operations (i.e. 'GET
/// Home/Index').
#[allow(dead_code)]
pub(crate) const OPERATION_NAME: ContextTagKey = ContextTagKey::new("ai.operation.name");

/// The unique identifier of the telemetry item's immediate parent.
#[allow(dead_code)]
pub(crate) const OPERATION_PARENT_ID: ContextTagKey = ContextTagKey::new("ai.operation.parentId");

/// Name of synthetic source. Some telemetry from the application may represent a synthetic
/// traffic. It may be web crawler indexing the web site, site availability tests or traces from
/// diagnostic libraries like Application Insights SDK itself.
#[allow(dead_code)]
pub(crate) const OPERATION_SYNTHETIC_SOURCE: ContextTagKey =
    ContextTagKey::new("ai.operation.syntheticSource");

/// The correlation vector is a light weight vector clock which can be used to identify and order
/// related events across clients and services.
#[allow(dead_code)]
pub(crate) const OPERATION_CORRELATION_VECTOR: ContextTagKey =
    ContextTagKey::new("ai.operation.correlationVector");

/// Session ID - the instance of the user's interaction with the app. Information in the session
/// context fields is always about the end user. When telemetry is sent from a service, the session
/// context is about the user that initiated the operation in the service.
#[allow(dead_code)]
pub(crate) const SESSION_ID: ContextTagKey = ContextTagKey::new("ai.session.id");

/// Boolean value indicating whether the session identified by ai.session.id is first for the user
/// or not.
#[allow(dead_code)]
pub(crate) const SESSION_IS_FIRST: ContextTagKey = ContextTagKey::new("ai.session.isFirst");

/// In multi-tenant applications this is the account ID or name which the user is acting with.
/// Examples may be subscription ID for Azure portal or blog name blogging platform.
#[allow(dead_code)]
pub(crate) const USER_ACCOUNT_ID: ContextTagKey = ContextTagKey::new("ai.user.accountId");

/// Anonymous user id. Represents the end user of the application. When telemetry is sent from a
/// service, the user context is about the user that initiated the operation in the service.
#[allow(dead_code)]
pub(crate) const USER_ID: ContextTagKey = ContextTagKey::new("ai.user.id");

/// Authenticated user id. The opposite of ai.user.id, this represents the user with a friendly
/// name. Since it's PII information it is not collected by default by most SDKs.
#[allow(dead_code)]
pub(crate) const USER_AUTH_USER_ID: ContextTagKey = ContextTagKey::new("ai.user.authUserId");

/// Name of the role the application is a part of. Maps directly to the role name in azure.
#[allow(dead_code)]
pub(crate) const CLOUD_ROLE: ContextTagKey = ContextTagKey::new("ai.cloud.role");

/// Name of the instance where the application is running. Computer name for on-premisis, instance
/// name for Azure.
#[allow(dead_code)]
pub(crate) const CLOUD_ROLE_INSTANCE: ContextTagKey = ContextTagKey::new("ai.cloud.roleInstance");

/// SDK version. See
/// https://github.com/Microsoft/ApplicationInsights-Home/blob/master/SDK-AUTHORING.md#sdk-version-specification
/// for information.
#[allow(dead_code)]
pub(crate) const INTERNAL_SDK_VERSION: ContextTagKey = ContextTagKey::new("ai.internal.sdkVersion");

/// Agent version. Used to indicate the version of StatusMonitor installed on the computer if it is
/// used for data collection.
#[allow(dead_code)]
pub(crate) const INTERNAL_AGENT_VERSION: ContextTagKey =
    ContextTagKey::new("ai.internal.agentVersion");

/// This is the node name used for billing purposes. Use it to override the standard detection of
/// nodes.
#[allow(dead_code)]
pub(crate) const INTERNAL_NODE_NAME: ContextTagKey = ContextTagKey::new("ai.internal.nodeName");
