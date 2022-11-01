use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub(crate) struct ContextTagKey {
    key: &'static str,
    max_len: usize,
}

impl ContextTagKey {
    const fn new(key: &'static str, max_len: usize) -> Self {
        Self { key, max_len }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct Tags(BTreeMap<&'static str, String>);

impl Tags {
    pub(crate) fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub(crate) fn insert(&mut self, key: ContextTagKey, mut value: String) -> Option<String> {
        value.truncate(key.max_len);
        self.0.insert(key.key, value)
    }

    #[cfg(test)]
    pub(crate) fn get(&self, key: &ContextTagKey) -> Option<&String> {
        self.0.get(key.key)
    }

    #[cfg(feature = "metrics")]
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

macro_rules! context_tag_keys {
    ($($(#[doc = $doc:expr])+ $var:ident($name:literal, $max_len:literal),)*) => {
        /// # Attributes for Application Insights context fields
        ///
        /// You can use these attributes in spans to set any of the [Application Insights context
        /// fields].
        ///
        /// [Application Insights context fields]: https://docs.microsoft.com/en-us/azure/azure-monitor/app/data-model-context
        ///
        /// ## Usage
        ///
        /// ```rust
        /// use opentelemetry::{global, trace::Tracer as _};
        /// use opentelemetry_application_insights::attrs as ai;
        ///
        /// let tracer = global::tracer("my-component");
        /// let _span = tracer
        ///     .span_builder("span-name")
        ///     .with_attributes(vec![
        ///         ai::SESSION_ID.string("42"),
        ///         ai::DEVICE_LOCALE.string("en-GB"),
        ///     ])
        ///     .start(&tracer);
        /// ```
        pub mod attrs {
            $($(#[doc = $doc])+
            pub const $var: opentelemetry::Key = opentelemetry::Key::from_static_str($name);)*

            /// Name for a custom event recorded with special name "ai.custom".
            pub const CUSTOM_EVENT_NAME: opentelemetry::Key =
                opentelemetry::Key::from_static_str("ai.customEvent.name");
        }

        $($(#[doc = $doc])+
        pub(crate) const $var: ContextTagKey = ContextTagKey::new($name, $max_len);)*

        pub(crate) static TAG_KEY_LOOKUP: Lazy<BTreeMap<opentelemetry::Key, ContextTagKey>> = Lazy::new(|| {
            vec![
                $((attrs::$var, $var),)*
            ]
            .into_iter()
            .collect()
        });
    }
}

context_tag_keys! {
    /// Application version. Information in the application context fields is always about the
    /// application that is sending the telemetry.
    APPLICATION_VERSION("ai.application.ver", 1024),

    /// Unique client device id. Computer name in most cases.
    DEVICE_ID("ai.device.id", 1024),

    /// Device locale using &lt;language&gt;-&lt;REGION&gt; pattern, following RFC 5646. Example
    /// 'en-US'.
    DEVICE_LOCALE("ai.device.locale", 64),

    /// Model of the device the end user of the application is using. Used for client scenarios. If
    /// this field is empty then it is derived from the user agent.
    DEVICE_MODEL("ai.device.model", 256),

    /// Client device OEM name taken from the browser.
    DEVICE_OEM_NAME("ai.device.oemName", 256),

    /// Operating system name and version of the device the end user of the application is using.
    /// If this field is empty then it is derived from the user agent. Example 'Windows 10 Pro
    /// 10.0.10586.0'
    DEVICE_OS_VERSION("ai.device.osVersion", 256),

    /// The type of the device the end user of the application is using. Used primarily to
    /// distinguish JavaScript telemetry from server side telemetry. Examples: 'PC', 'Phone',
    /// 'Browser'. 'PC' is the default value.
    DEVICE_TYPE("ai.device.type", 64),

    /// The IP address of the client device. IPv4 and IPv6 are supported. Information in the
    /// location context fields is always about the end user. When telemetry is sent from a
    /// service, the location context is about the user that initiated the operation in the
    /// service.
    LOCATION_IP("ai.location.ip", 46),

    /// The country of the client device. If any of Country, Province, or City is specified, those
    /// values will be preferred over geolocation of the IP address field. Information in the
    /// location context fields is always about the end user. When telemetry is sent from a
    /// service, the location context is about the user that initiated the operation in the
    /// service.
    LOCATION_COUNTRY("ai.location.country", 256),

    /// The province/state of the client device. If any of Country, Province, or City is specified,
    /// those values will be preferred over geolocation of the IP address field. Information in the
    /// location context fields is always about the end user. When telemetry is sent from a
    /// service, the location context is about the user that initiated the operation in the
    /// service.
    LOCATION_PROVINCE("ai.location.province", 256),

    /// The city of the client device. If any of Country, Province, or City is specified, those
    /// values will be preferred over geolocation of the IP address field. Information in the
    /// location context fields is always about the end user. When telemetry is sent from a
    /// service, the location context is about the user that initiated the operation in the
    /// service.
    LOCATION_CITY("ai.location.city", 256),

    /// A unique identifier for the operation instance. The operation.id is created by either a
    /// request or a page view. All other telemetry sets this to the value for the containing
    /// request or page view. Operation.id is used for finding all the telemetry items for a
    /// specific operation instance.
    OPERATION_ID("ai.operation.id", 128),

    /// The name (group) of the operation. The operation.name is created by either a request or a
    /// page view. All other telemetry items set this to the value for the containing request or
    /// page view. Operation.name is used for finding all the telemetry items for a group of
    /// operations (i.e. 'GET Home/Index').
    OPERATION_NAME("ai.operation.name", 1024),

    /// The unique identifier of the telemetry item's immediate parent.
    OPERATION_PARENT_ID("ai.operation.parentId", 128),

    /// Name of synthetic source. Some telemetry from the application may represent a synthetic
    /// traffic. It may be web crawler indexing the web site, site availability tests or traces
    /// from diagnostic libraries like Application Insights SDK itself.
    OPERATION_SYNTHETIC_SOURCE("ai.operation.syntheticSource", 1024),

    /// The correlation vector is a light weight vector clock which can be used to identify and
    /// order related events across clients and services.
    OPERATION_CORRELATION_VECTOR("ai.operation.correlationVector", 64),

    /// Session ID - the instance of the user's interaction with the app. Information in the
    /// session context fields is always about the end user. When telemetry is sent from a service,
    /// the session context is about the user that initiated the operation in the service.
    SESSION_ID("ai.session.id", 64),

    /// Boolean value indicating whether the session identified by ai.session.id is first for the
    /// user or not.
    SESSION_IS_FIRST("ai.session.isFirst", 5),

    /// In multi-tenant applications this is the account ID or name which the user is acting with.
    /// Examples may be subscription ID for Azure portal or blog name blogging platform.
    USER_ACCOUNT_ID("ai.user.accountId", 1024),

    /// Anonymous user id. Represents the end user of the application. When telemetry is sent from
    /// a service, the user context is about the user that initiated the operation in the service.
    USER_ID("ai.user.id", 128),

    /// Authenticated user id. The opposite of ai.user.id, this represents the user with a friendly
    /// name. Since it's PII information it is not collected by default by most SDKs.
    USER_AUTH_USER_ID("ai.user.authUserId", 1024),

    /// Name of the role the application is a part of. Maps directly to the role name in azure.
    CLOUD_ROLE("ai.cloud.role", 256),

    /// Name of the instance where the application is running. Computer name for on-premisis,
    /// instance name for Azure.
    CLOUD_ROLE_INSTANCE("ai.cloud.roleInstance", 256),

    /// SDK version. See
    /// <https://github.com/Microsoft/ApplicationInsights-Home/blob/master/SDK-AUTHORING.md#sdk-version-specification>
    /// for information.
    INTERNAL_SDK_VERSION("ai.internal.sdkVersion", 64),

    /// Agent version. Used to indicate the version of StatusMonitor installed on the computer if
    /// it is used for data collection.
    INTERNAL_AGENT_VERSION("ai.internal.agentVersion", 64),

    /// This is the node name used for billing purposes. Use it to override the standard detection
    /// of nodes.
    INTERNAL_NODE_NAME("ai.internal.nodeName", 256),
}
