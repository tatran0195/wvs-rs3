//! FFI function declarations for FlexNet Publisher native library.
//!
//! These bindings define the C ABI interface to the FlexNet licensing library.
//! When the `ffi` feature is enabled, the actual DLL/SO is loaded at runtime.
//! When `mock` feature is enabled, a mock implementation is used instead.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::sync::Arc;

use thiserror::Error;
use tracing;

/// Errors from FFI operations
#[derive(Debug, Error)]
pub enum FfiError {
    /// Failed to load the native library
    #[error("Failed to load FlexNet library: {0}")]
    LibraryLoadError(String),

    /// FFI function call failed
    #[error("FlexNet FFI call failed: {function} returned {code}")]
    FunctionCallError {
        /// The function that failed
        function: String,
        /// The error code returned
        code: i32,
    },

    /// String conversion error
    #[error("String conversion error: {0}")]
    StringConversionError(String),

    /// Library not initialized
    #[error("FlexNet library not initialized")]
    NotInitialized,

    /// Checkout failed
    #[error("License checkout failed: {0}")]
    CheckoutFailed(String),

    /// Checkin failed
    #[error("License checkin failed: {0}")]
    CheckinFailed(String),
}

/// FlexNet status codes
pub const FLEXNET_OK: c_int = 0;
/// No more licenses available
pub const FLEXNET_NO_LICENSE: c_int = -1;
/// Feature not found
pub const FLEXNET_FEATURE_NOT_FOUND: c_int = -2;
/// License expired
pub const FLEXNET_EXPIRED: c_int = -3;
/// Server error
pub const FLEXNET_SERVER_ERROR: c_int = -4;

/// Opaque handle for a license checkout
pub type FlexNetHandle = *mut std::ffi::c_void;

/// FFI function type definitions matching the FlexNet Publisher C API
pub type FnFlexNetInit = unsafe extern "C" fn(license_file: *const c_char) -> c_int;
/// Cleanup function
pub type FnFlexNetCleanup = unsafe extern "C" fn() -> c_int;
/// Checkout a feature license
pub type FnFlexNetCheckout = unsafe extern "C" fn(
    feature: *const c_char,
    version: *const c_char,
    count: c_int,
    handle_out: *mut FlexNetHandle,
) -> c_int;
/// Checkin a feature license
pub type FnFlexNetCheckin = unsafe extern "C" fn(handle: FlexNetHandle) -> c_int;
/// Get total seats for a feature
pub type FnFlexNetGetTotalSeats =
    unsafe extern "C" fn(feature: *const c_char, count_out: *mut c_int) -> c_int;
/// Get available seats for a feature
pub type FnFlexNetGetAvailableSeats =
    unsafe extern "C" fn(feature: *const c_char, count_out: *mut c_int) -> c_int;
/// Get last error message
pub type FnFlexNetGetLastError = unsafe extern "C" fn() -> *const c_char;

/// Trait for FlexNet bindings abstraction (enables mock and real implementations)
pub trait FlexNetBindings: Send + Sync {
    /// Initialize the FlexNet library with a license file path
    fn init(&self, license_file: &str) -> Result<(), FfiError>;

    /// Cleanup and release the FlexNet library
    fn cleanup(&self) -> Result<(), FfiError>;

    /// Checkout a license for a feature
    fn checkout(&self, feature: &str, version: &str) -> Result<String, FfiError>;

    /// Checkin a license by its token handle
    fn checkin(&self, token: &str) -> Result<(), FfiError>;

    /// Get the total number of seats for a feature
    fn get_total_seats(&self, feature: &str) -> Result<i32, FfiError>;

    /// Get the number of available seats for a feature
    fn get_available_seats(&self, feature: &str) -> Result<i32, FfiError>;

    /// Get the last error message from the library
    fn get_last_error(&self) -> Option<String>;
}

/// Mock implementation of FlexNet bindings for testing
#[cfg(feature = "mock")]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Mock FlexNet bindings for development and testing
    #[derive(Debug)]
    pub struct MockFlexNetBindings {
        /// Whether the library has been initialized
        initialized: Mutex<bool>,
        /// Active checkouts: token -> feature
        checkouts: Mutex<HashMap<String, String>>,
        /// Total seats per feature
        total_seats: Mutex<HashMap<String, i32>>,
        /// Counter for generating unique tokens
        token_counter: Mutex<u64>,
    }

    impl MockFlexNetBindings {
        /// Create a new mock bindings instance
        pub fn new() -> Self {
            Self {
                initialized: Mutex::new(false),
                checkouts: Mutex::new(HashMap::new()),
                total_seats: Mutex::new(HashMap::new()),
                token_counter: Mutex::new(0),
            }
        }

        /// Set the total seats for a feature (for testing)
        pub fn set_total_seats(&self, feature: &str, seats: i32) {
            let mut total = self.total_seats.lock().unwrap_or_else(|e| e.into_inner());
            total.insert(feature.to_string(), seats);
        }
    }

    impl Default for MockFlexNetBindings {
        fn default() -> Self {
            Self::new()
        }
    }

    impl FlexNetBindings for MockFlexNetBindings {
        fn init(&self, _license_file: &str) -> Result<(), FfiError> {
            let mut init = self.initialized.lock().unwrap_or_else(|e| e.into_inner());
            *init = true;
            tracing::info!("[MockFlexNet] Initialized");
            Ok(())
        }

        fn cleanup(&self) -> Result<(), FfiError> {
            let mut init = self.initialized.lock().unwrap_or_else(|e| e.into_inner());
            let mut checkouts = self.checkouts.lock().unwrap_or_else(|e| e.into_inner());
            checkouts.clear();
            *init = false;
            tracing::info!("[MockFlexNet] Cleaned up");
            Ok(())
        }

        fn checkout(&self, feature: &str, _version: &str) -> Result<String, FfiError> {
            let init = self.initialized.lock().unwrap_or_else(|e| e.into_inner());
            if !*init {
                return Err(FfiError::NotInitialized);
            }
            drop(init);

            let total = self.total_seats.lock().unwrap_or_else(|e| e.into_inner());
            let max_seats = total.get(feature).copied().unwrap_or(10);
            drop(total);

            let mut checkouts = self.checkouts.lock().unwrap_or_else(|e| e.into_inner());
            let active_for_feature =
                checkouts.values().filter(|f| f.as_str() == feature).count() as i32;

            if active_for_feature >= max_seats {
                return Err(FfiError::CheckoutFailed(format!(
                    "No seats available for feature '{}' ({}/{})",
                    feature, active_for_feature, max_seats
                )));
            }

            let mut counter = self.token_counter.lock().unwrap_or_else(|e| e.into_inner());
            *counter += 1;
            let token = format!("MOCK-{}-{}", feature, *counter);
            checkouts.insert(token.clone(), feature.to_string());

            tracing::info!(
                "[MockFlexNet] Checked out feature '{}', token='{}', active={}/{}",
                feature,
                token,
                active_for_feature + 1,
                max_seats
            );

            Ok(token)
        }

        fn checkin(&self, token: &str) -> Result<(), FfiError> {
            let init = self.initialized.lock().unwrap_or_else(|e| e.into_inner());
            if !*init {
                return Err(FfiError::NotInitialized);
            }
            drop(init);

            let mut checkouts = self.checkouts.lock().unwrap_or_else(|e| e.into_inner());
            if checkouts.remove(token).is_some() {
                tracing::info!("[MockFlexNet] Checked in token='{}'", token);
                Ok(())
            } else {
                Err(FfiError::CheckinFailed(format!(
                    "Token '{}' not found",
                    token
                )))
            }
        }

        fn get_total_seats(&self, feature: &str) -> Result<i32, FfiError> {
            let init = self.initialized.lock().unwrap_or_else(|e| e.into_inner());
            if !*init {
                return Err(FfiError::NotInitialized);
            }
            drop(init);

            let total = self.total_seats.lock().unwrap_or_else(|e| e.into_inner());
            Ok(total.get(feature).copied().unwrap_or(10))
        }

        fn get_available_seats(&self, feature: &str) -> Result<i32, FfiError> {
            let init = self.initialized.lock().unwrap_or_else(|e| e.into_inner());
            if !*init {
                return Err(FfiError::NotInitialized);
            }
            drop(init);

            let total = self.total_seats.lock().unwrap_or_else(|e| e.into_inner());
            let max_seats = total.get(feature).copied().unwrap_or(10);
            drop(total);

            let checkouts = self.checkouts.lock().unwrap_or_else(|e| e.into_inner());
            let active = checkouts.values().filter(|f| f.as_str() == feature).count() as i32;

            Ok(max_seats - active)
        }

        fn get_last_error(&self) -> Option<String> {
            None
        }
    }
}

/// Real FFI implementation using libloading (requires `ffi` feature)
#[cfg(feature = "ffi")]
pub mod real {
    use super::*;
    use libloading::{Library, Symbol};
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Real FlexNet bindings loaded from a native shared library
    pub struct RealFlexNetBindings {
        /// The loaded native library
        library: Option<Library>,
        /// Active handles for checkin
        handles: Mutex<HashMap<String, usize>>,
        /// Handle counter
        handle_counter: Mutex<u64>,
    }

    impl std::fmt::Debug for RealFlexNetBindings {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("RealFlexNetBindings")
                .field("loaded", &self.library.is_some())
                .finish()
        }
    }

    impl RealFlexNetBindings {
        /// Load FlexNet bindings from a shared library path
        pub fn load(library_path: &str) -> Result<Self, FfiError> {
            let library = unsafe {
                Library::new(library_path).map_err(|e| FfiError::LibraryLoadError(e.to_string()))?
            };

            Ok(Self {
                library: Some(library),
                handles: Mutex::new(HashMap::new()),
                handle_counter: Mutex::new(0),
            })
        }
    }

    impl FlexNetBindings for RealFlexNetBindings {
        fn init(&self, license_file: &str) -> Result<(), FfiError> {
            let lib = self.library.as_ref().ok_or(FfiError::NotInitialized)?;
            let c_path = CString::new(license_file)
                .map_err(|e| FfiError::StringConversionError(e.to_string()))?;

            unsafe {
                let func: Symbol<FnFlexNetInit> = lib
                    .get(b"flexnet_init")
                    .map_err(|e| FfiError::LibraryLoadError(e.to_string()))?;
                let result = func(c_path.as_ptr());
                if result != FLEXNET_OK {
                    return Err(FfiError::FunctionCallError {
                        function: "flexnet_init".to_string(),
                        code: result,
                    });
                }
            }
            Ok(())
        }

        fn cleanup(&self) -> Result<(), FfiError> {
            let lib = self.library.as_ref().ok_or(FfiError::NotInitialized)?;
            unsafe {
                let func: Symbol<FnFlexNetCleanup> = lib
                    .get(b"flexnet_cleanup")
                    .map_err(|e| FfiError::LibraryLoadError(e.to_string()))?;
                let result = func();
                if result != FLEXNET_OK {
                    return Err(FfiError::FunctionCallError {
                        function: "flexnet_cleanup".to_string(),
                        code: result,
                    });
                }
            }
            Ok(())
        }

        fn checkout(&self, feature: &str, version: &str) -> Result<String, FfiError> {
            let lib = self.library.as_ref().ok_or(FfiError::NotInitialized)?;
            let c_feature = CString::new(feature)
                .map_err(|e| FfiError::StringConversionError(e.to_string()))?;
            let c_version = CString::new(version)
                .map_err(|e| FfiError::StringConversionError(e.to_string()))?;

            let mut handle: FlexNetHandle = std::ptr::null_mut();

            unsafe {
                let func: Symbol<FnFlexNetCheckout> = lib
                    .get(b"flexnet_checkout")
                    .map_err(|e| FfiError::LibraryLoadError(e.to_string()))?;
                let result = func(c_feature.as_ptr(), c_version.as_ptr(), 1, &mut handle);
                if result != FLEXNET_OK {
                    return Err(FfiError::CheckoutFailed(
                        self.get_last_error()
                            .unwrap_or_else(|| format!("error code: {}", result)),
                    ));
                }
            }

            let mut counter = self
                .handle_counter
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            *counter += 1;
            let token = format!("FLEXNET-{}-{}", feature, *counter);
            drop(counter);

            let mut handles = self.handles.lock().unwrap_or_else(|e| e.into_inner());
            handles.insert(token.clone(), handle as usize);

            Ok(token)
        }

        fn checkin(&self, token: &str) -> Result<(), FfiError> {
            let lib = self.library.as_ref().ok_or(FfiError::NotInitialized)?;

            let mut handles = self.handles.lock().unwrap_or_else(|e| e.into_inner());
            let handle_val = handles
                .remove(token)
                .ok_or_else(|| FfiError::CheckinFailed(format!("Token '{}' not found", token)))?;
            drop(handles);

            let handle = handle_val as FlexNetHandle;

            unsafe {
                let func: Symbol<FnFlexNetCheckin> = lib
                    .get(b"flexnet_checkin")
                    .map_err(|e| FfiError::LibraryLoadError(e.to_string()))?;
                let result = func(handle);
                if result != FLEXNET_OK {
                    return Err(FfiError::CheckinFailed(
                        self.get_last_error()
                            .unwrap_or_else(|| format!("error code: {}", result)),
                    ));
                }
            }
            Ok(())
        }

        fn get_total_seats(&self, feature: &str) -> Result<i32, FfiError> {
            let lib = self.library.as_ref().ok_or(FfiError::NotInitialized)?;
            let c_feature = CString::new(feature)
                .map_err(|e| FfiError::StringConversionError(e.to_string()))?;
            let mut count: c_int = 0;

            unsafe {
                let func: Symbol<FnFlexNetGetTotalSeats> = lib
                    .get(b"flexnet_get_total_seats")
                    .map_err(|e| FfiError::LibraryLoadError(e.to_string()))?;
                let result = func(c_feature.as_ptr(), &mut count);
                if result != FLEXNET_OK {
                    return Err(FfiError::FunctionCallError {
                        function: "flexnet_get_total_seats".to_string(),
                        code: result,
                    });
                }
            }
            Ok(count)
        }

        fn get_available_seats(&self, feature: &str) -> Result<i32, FfiError> {
            let lib = self.library.as_ref().ok_or(FfiError::NotInitialized)?;
            let c_feature =
                CString::new(feature).map_err(|e| FfiError::LibraryLoadError(e.to_string()))?;
            let mut count: c_int = 0;

            unsafe {
                let func: Symbol<FnFlexNetGetAvailableSeats> = lib
                    .get(b"flexnet_get_available_seats")
                    .map_err(|e| FfiError::LibraryLoadError(e.to_string()))?;
                let result = func(c_feature.as_ptr(), &mut count);
                if result != FLEXNET_OK {
                    return Err(FfiError::FunctionCallError {
                        function: "flexnet_get_available_seats".to_string(),
                        code: result,
                    });
                }
            }
            Ok(count)
        }

        fn get_last_error(&self) -> Option<String> {
            let lib = self.library.as_ref()?;
            unsafe {
                let func: Result<Symbol<FnFlexNetGetLastError>, _> =
                    lib.get(b"flexnet_get_last_error");
                match func {
                    Ok(f) => {
                        let ptr = f();
                        if ptr.is_null() {
                            None
                        } else {
                            Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
                        }
                    }
                    Err(_) => None,
                }
            }
        }
    }
}
