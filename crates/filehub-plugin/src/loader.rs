//! Dynamic plugin loader using `libloading` (feature-gated).

#[cfg(feature = "dynamic")]
pub mod dynamic_loader {
    use std::path::Path;
    use std::sync::Arc;

    use tracing::{error, info};

    use crate::registry::Plugin;
    use filehub_core::error::AppError;

    /// Type of the plugin creation function exported by dynamic plugins.
    ///
    /// Dynamic plugins must export: `extern "C" fn create_plugin() -> *mut dyn Plugin`
    pub type CreatePluginFn = unsafe extern "C" fn() -> *mut dyn Plugin;

    /// Loads a plugin from a shared library (.so / .dll / .dylib).
    pub struct DynamicLoader {
        /// Loaded libraries (kept alive for the lifetime of the loader).
        _libraries: Vec<libloading::Library>,
    }

    impl DynamicLoader {
        /// Creates a new dynamic loader.
        pub fn new() -> Self {
            Self {
                _libraries: Vec::new(),
            }
        }

        /// Loads a plugin from the given shared library path.
        ///
        /// # Safety
        /// This function loads arbitrary code from a shared library.
        /// Only load trusted plugins.
        pub unsafe fn load_from_path(&mut self, path: &Path) -> Result<Arc<dyn Plugin>, AppError> {
            let lib = libloading::Library::new(path).map_err(|e| {
                AppError::internal(format!(
                    "Failed to load plugin library '{}': {}",
                    path.display(),
                    e
                ))
            })?;

            let create_fn: libloading::Symbol<CreatePluginFn> =
                lib.get(b"create_plugin").map_err(|e| {
                    AppError::internal(format!(
                        "Plugin '{}' missing 'create_plugin' symbol: {}",
                        path.display(),
                        e
                    ))
                })?;

            let raw_plugin = create_fn();
            let plugin = Arc::from_raw(raw_plugin);

            info!(
                path = %path.display(),
                "Dynamic plugin loaded"
            );

            self._libraries.push(lib);

            Ok(plugin)
        }
    }

    impl std::fmt::Debug for DynamicLoader {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("DynamicLoader")
                .field("loaded_count", &self._libraries.len())
                .finish()
        }
    }
}

/// Stub loader when dynamic feature is not enabled.
#[cfg(not(feature = "dynamic"))]
pub mod dynamic_loader {
    /// Stub dynamic loader.
    #[derive(Debug)]
    pub struct DynamicLoader;

    impl DynamicLoader {
        /// Creates a stub loader.
        pub fn new() -> Self {
            Self
        }
    }

    impl Default for DynamicLoader {
        fn default() -> Self {
            Self::new()
        }
    }
}

pub use dynamic_loader::DynamicLoader;
