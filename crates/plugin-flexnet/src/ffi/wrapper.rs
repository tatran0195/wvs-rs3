//! Safe Rust wrapper around the license manager.
//!
//! Abstracts over both the real FFI (`LicenseProxyApi`) and the
//! mock implementation, providing a unified interface for the
//! rest of the plugin.

use std::ffi::{CStr, CString, c_char};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use tracing::{info, warn};

use super::bindings::{LM_SUCCESS, LicenseManagerCtx, LicenseProxyApi};
use super::mock::MockLicenseManager;

/// Unified license manager that wraps either real FFI or mock.
///
/// The auth flow:
/// - Login: create session → `checkout(feature, session_id)`
/// - Logout: `checkin(feature, session_id)` → destroy session
#[derive(Debug)]
pub enum LicenseManagerWrapper {
    /// Real FFI implementation loaded from DLL
    Real(RealLicenseManager),
    /// Mock implementation for development/testing
    Mock(MockLicenseManager),
}

/// Real license manager backed by `license_proxy.dll`
#[derive(Debug)]
pub struct RealLicenseManager {
    /// Loaded API function pointers
    api: Arc<LicenseProxyApi>,
    /// Native context pointer
    ctx: *mut LicenseManagerCtx,
}

// Safety: The ctx pointer is only accessed through the API functions
// which are thread-safe according to the DLL documentation.
unsafe impl Send for RealLicenseManager {}
unsafe impl Sync for RealLicenseManager {}

impl RealLicenseManager {
    /// Load the DLL and create a new license manager context
    pub fn new(dll_path: PathBuf) -> Result<Self> {
        info!("Loading license manager from {:?}", dll_path);

        let path_str = dll_path
            .to_str()
            .ok_or_else(|| anyhow!("Invalid DLL path: {:?}", dll_path))?;

        let api = LicenseProxyApi::load(path_str)
            .map_err(|e| anyhow!("Failed to load license_proxy DLL: {}", e))?;

        let mut ctx: *mut LicenseManagerCtx = std::ptr::null_mut();

        unsafe {
            let res = (api.create)(&mut ctx);
            if res != LM_SUCCESS {
                return Err(anyhow!("LM_Create failed: error code {}", res));
            }
        }

        if ctx.is_null() {
            return Err(anyhow!("LM_Create returned null context"));
        }

        info!("License manager DLL loaded successfully");

        Ok(Self {
            api: Arc::new(api),
            ctx,
        })
    }
}

impl Drop for RealLicenseManager {
    fn drop(&mut self) {
        if !self.ctx.is_null() {
            info!("Destroying license manager context");
            unsafe {
                (self.api.destroy)(self.ctx);
            }
            self.ctx = std::ptr::null_mut();
        }
    }
}

impl LicenseManagerWrapper {
    /// Create a real license manager from a DLL path
    pub fn new_real(dll_path: PathBuf) -> Result<Self> {
        let real = RealLicenseManager::new(dll_path)?;
        Ok(Self::Real(real))
    }

    /// Create a mock license manager
    pub fn new_mock() -> Self {
        Self::Mock(MockLicenseManager::new())
    }

    /// Create the appropriate implementation based on configuration.
    ///
    /// If the DLL path exists, loads the real implementation.
    /// Otherwise falls back to mock (if `mock` feature is enabled).
    pub fn create(dll_path: Option<PathBuf>) -> Result<Self> {
        let path = dll_path.unwrap_or_else(|| PathBuf::from("license_proxy.dll"));

        if path.exists() {
            info!(
                "License DLL found at {:?}, loading real implementation",
                path
            );
            Self::new_real(path)
        } else {
            #[cfg(feature = "mock")]
            {
                warn!(
                    "License DLL not found at {:?}, using mock implementation",
                    path
                );
                Ok(Self::new_mock())
            }
            #[cfg(not(feature = "mock"))]
            {
                Err(anyhow!(
                    "License DLL not found at {:?} and mock feature is disabled",
                    path
                ))
            }
        }
    }

    /// Initialize the license manager.
    ///
    /// Optionally provide an override path for the license file.
    pub fn initialize(&self, override_path: Option<&str>) -> Result<()> {
        match self {
            Self::Real(real) => {
                let c_path = match override_path {
                    Some(p) => {
                        Some(CString::new(p).map_err(|e| anyhow!("Invalid override path: {}", e))?)
                    }
                    None => None,
                };
                let p_ptr = c_path
                    .as_ref()
                    .map(|s| s.as_ptr())
                    .unwrap_or(std::ptr::null());

                unsafe {
                    let res = (real.api.initialize)(real.ctx, p_ptr);
                    if res != LM_SUCCESS {
                        return Err(anyhow!("LM_Initialize failed: error code {}", res));
                    }
                }
                info!("License manager initialized (real)");
                Ok(())
            }
            Self::Mock(mock) => {
                let res = mock.initialize(override_path);
                if res != 0 {
                    return Err(anyhow!("Mock initialize failed: {}", res));
                }
                Ok(())
            }
        }
    }

    /// Checkout a license seat for a session.
    ///
    /// Called after session creation during login.
    ///
    /// # Arguments
    /// * `feature` — The license feature name (e.g., "suzuki_filehub")
    /// * `session_id` — The session UUID as a string
    pub fn checkout(&self, feature: &str, session_id: &str) -> Result<()> {
        match self {
            Self::Real(real) => {
                let c_feature =
                    CString::new(feature).map_err(|e| anyhow!("Invalid feature name: {}", e))?;
                let c_session =
                    CString::new(session_id).map_err(|e| anyhow!("Invalid session_id: {}", e))?;

                unsafe {
                    let res =
                        (real.api.check_out)(real.ctx, c_feature.as_ptr(), c_session.as_ptr());
                    if res != LM_SUCCESS {
                        return Err(anyhow!(
                            "LM_CheckOut failed for feature='{}', session='{}': error code {}",
                            feature,
                            session_id,
                            res
                        ));
                    }
                }
                info!(
                    "License checked out: feature='{}', session='{}'",
                    feature, session_id
                );
                Ok(())
            }
            Self::Mock(mock) => {
                let res = mock.checkout(feature, session_id);
                if res != 0 {
                    return Err(anyhow!(
                        "Mock checkout failed for feature='{}', session='{}'",
                        feature,
                        session_id
                    ));
                }
                Ok(())
            }
        }
    }

    /// Checkin (release) a license seat for a session.
    ///
    /// Called during logout or session termination.
    ///
    /// # Arguments
    /// * `feature` — The license feature name
    /// * `session_id` — The session UUID as a string
    pub fn checkin(&self, feature: &str, session_id: &str) -> Result<()> {
        match self {
            Self::Real(real) => {
                let c_feature =
                    CString::new(feature).map_err(|e| anyhow!("Invalid feature name: {}", e))?;
                let c_session =
                    CString::new(session_id).map_err(|e| anyhow!("Invalid session_id: {}", e))?;

                unsafe {
                    let res = (real.api.check_in)(real.ctx, c_feature.as_ptr(), c_session.as_ptr());
                    if res != LM_SUCCESS {
                        warn!(
                            "LM_CheckIn warning for feature='{}', session='{}': error code {}",
                            feature, session_id, res
                        );
                        // Checkin failures are non-fatal — the seat may have already been released
                    }
                }
                info!(
                    "License checked in: feature='{}', session='{}'",
                    feature, session_id
                );
                Ok(())
            }
            Self::Mock(mock) => {
                let res = mock.checkin(feature, session_id);
                if res != 0 {
                    warn!(
                        "Mock checkin warning for feature='{}', session='{}': code {}",
                        feature, session_id, res
                    );
                }
                Ok(())
            }
        }
    }

    /// Get the token pool status for a feature.
    ///
    /// Returns `(total_seats, used_seats)`.
    pub fn get_token_pool(&self, feature: &str) -> Result<(i32, i32)> {
        match self {
            Self::Real(real) => {
                let c_feature =
                    CString::new(feature).map_err(|e| anyhow!("Invalid feature name: {}", e))?;
                let mut total: i32 = 0;
                let mut used: i32 = 0;

                unsafe {
                    let res = (real.api.get_token_pool)(
                        real.ctx,
                        c_feature.as_ptr(),
                        &mut total,
                        &mut used,
                    );
                    if res != LM_SUCCESS {
                        return Err(anyhow!(
                            "LM_GetTokenPool failed for '{}': error code {}",
                            feature,
                            res
                        ));
                    }
                }
                Ok((total, used))
            }
            Self::Mock(mock) => {
                let (res, total, used) = mock.get_token_pool(feature);
                if res != 0 {
                    return Err(anyhow!(
                        "Mock get_token_pool failed for '{}': code {}",
                        feature,
                        res
                    ));
                }
                Ok((total, used))
            }
        }
    }

    /// Check if this is a star (unlimited) license.
    pub fn is_star_license(&self) -> bool {
        match self {
            Self::Real(real) => {
                let mut is_star = false;
                unsafe {
                    (real.api.is_star_license)(real.ctx, &mut is_star);
                }
                is_star
            }
            Self::Mock(mock) => mock.is_star_license(),
        }
    }

    /// Get server info string.
    pub fn get_server_info(&self) -> String {
        match self {
            Self::Real(real) => {
                let mut buffer = vec![0u8; 256];
                unsafe {
                    let res = (real.api.get_server_info)(
                        real.ctx,
                        buffer.as_mut_ptr() as *mut c_char,
                        buffer.len() as i32,
                    );
                    if res == LM_SUCCESS {
                        return CStr::from_ptr(buffer.as_ptr() as *const c_char)
                            .to_string_lossy()
                            .into_owned();
                    }
                }
                "Unknown".to_string()
            }
            Self::Mock(mock) => mock.get_server_info(),
        }
    }

    /// Release all checked-out licenses.
    ///
    /// Called during shutdown or emergency release.
    pub fn release_all(&self) {
        match self {
            Self::Real(real) => {
                info!("Releasing all license checkouts");
                unsafe {
                    (real.api.release_all)(real.ctx);
                }
            }
            Self::Mock(mock) => {
                mock.release_all();
            }
        }
    }

    /// Check if using mock implementation
    pub fn is_mock(&self) -> bool {
        matches!(self, Self::Mock(_))
    }

    /// Access the mock for test configuration (panics if not mock)
    #[cfg(feature = "mock")]
    pub fn as_mock(&self) -> &MockLicenseManager {
        match self {
            Self::Mock(mock) => mock,
            Self::Real(_) => panic!("as_mock() called on real implementation"),
        }
    }
}
