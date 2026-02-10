//! Convenience macros for plugin development.

/// Macro for creating a simple plugin info struct.
///
/// # Example
/// ```rust,ignore
/// let info = plugin_info!(
///     id: "my-plugin",
///     name: "My Plugin",
///     version: "1.0.0",
///     description: "Does things",
///     author: "Dev"
/// );
/// ```
#[macro_export]
macro_rules! plugin_info {
    (
        id: $id:expr,
        name: $name:expr,
        version: $version:expr,
        description: $desc:expr,
        author: $author:expr
    ) => {
        $crate::prelude::PluginInfo {
            id: $id.to_string(),
            name: $name.to_string(),
            version: $version.to_string(),
            description: $desc.to_string(),
            author: $author.to_string(),
            hooks: Vec::new(),
            enabled: true,
            priority: 100,
        }
    };
    (
        id: $id:expr,
        name: $name:expr,
        version: $version:expr,
        description: $desc:expr,
        author: $author:expr,
        priority: $priority:expr
    ) => {
        $crate::prelude::PluginInfo {
            id: $id.to_string(),
            name: $name.to_string(),
            version: $version.to_string(),
            description: $desc.to_string(),
            author: $author.to_string(),
            hooks: Vec::new(),
            enabled: true,
            priority: $priority,
        }
    };
}

/// Macro for quickly building a `HookPayload`.
///
/// # Example
/// ```rust,ignore
/// let payload = hook_payload!(HookPoint::AfterUpload, {
///     "file_id" => json!("abc-123"),
///     "filename" => json!("test.pdf"),
/// });
/// ```
#[macro_export]
macro_rules! hook_payload {
    ($hook:expr) => {
        $crate::prelude::HookPayload::new($hook)
    };
    ($hook:expr, { $($key:expr => $value:expr),* $(,)? }) => {{
        let mut payload = $crate::prelude::HookPayload::new($hook);
        $(
            payload.data.insert($key.to_string(), $value);
        )*
        payload
    }};
    ($hook:expr, actor: $actor:expr, { $($key:expr => $value:expr),* $(,)? }) => {{
        let mut payload = $crate::prelude::HookPayload::new($hook).with_actor($actor);
        $(
            payload.data.insert($key.to_string(), $value);
        )*
        payload
    }};
}
