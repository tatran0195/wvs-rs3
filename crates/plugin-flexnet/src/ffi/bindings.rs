//! FFI function declarations for license_proxy.dll.
//!
//! Loads the native DLL at runtime using `libloading` and exposes
//! all license manager functions through safe Rust wrappers.

use std::ffi::{c_char, c_int, c_void};
use std::mem;
use std::sync::Arc;

/// Opaque context pointer for the license manager
pub type LicenseManagerCtx = c_void;

/// Result type from license manager functions
#[allow(non_camel_case_types)]
pub type LM_Result = c_int;

/// Success return code
pub const LM_SUCCESS: LM_Result = 0;

/// Loaded license proxy API with all function pointers.
///
/// Each field is a dynamically loaded symbol from the DLL.
/// The `_lib` field keeps the library alive for the lifetime of this struct.
#[derive(Clone, Debug)]
pub struct LicenseProxyApi {
    /// `LM_Create(out_ctx: *mut *mut LicenseManagerCtx) -> LM_Result`
    pub create: libloading::Symbol<
        'static,
        unsafe extern "C" fn(out_ctx: *mut *mut LicenseManagerCtx) -> LM_Result,
    >,

    /// `LM_Destroy(ctx: *mut LicenseManagerCtx) -> LM_Result`
    pub destroy:
        libloading::Symbol<'static, unsafe extern "C" fn(ctx: *mut LicenseManagerCtx) -> LM_Result>,

    /// `LM_Initialize(ctx: *mut LicenseManagerCtx, override_path: *const c_char) -> LM_Result`
    pub initialize: libloading::Symbol<
        'static,
        unsafe extern "C" fn(
            ctx: *mut LicenseManagerCtx,
            override_path: *const c_char,
        ) -> LM_Result,
    >,

    /// `LM_CheckOut(ctx, feature, session_id) -> LM_Result`
    pub check_out: libloading::Symbol<
        'static,
        unsafe extern "C" fn(
            ctx: *mut LicenseManagerCtx,
            feature: *const c_char,
            session_id: *const c_char,
        ) -> LM_Result,
    >,

    /// `LM_CheckIn(ctx, feature, session_id) -> LM_Result`
    pub check_in: libloading::Symbol<
        'static,
        unsafe extern "C" fn(
            ctx: *mut LicenseManagerCtx,
            feature: *const c_char,
            session_id: *const c_char,
        ) -> LM_Result,
    >,

    /// `LM_GetTokenPool(ctx, feature, out_total, out_used) -> LM_Result`
    pub get_token_pool: libloading::Symbol<
        'static,
        unsafe extern "C" fn(
            ctx: *mut LicenseManagerCtx,
            feature: *const c_char,
            out_total: *mut c_int,
            out_used: *mut c_int,
        ) -> LM_Result,
    >,

    /// `LM_IsStarLicense(ctx, out_is_star) -> LM_Result`
    pub is_star_license: libloading::Symbol<
        'static,
        unsafe extern "C" fn(ctx: *mut LicenseManagerCtx, out_is_star: *mut bool) -> LM_Result,
    >,

    /// `LM_GetServerInfo(ctx, out_buffer, buffer_size) -> LM_Result`
    pub get_server_info: libloading::Symbol<
        'static,
        unsafe extern "C" fn(
            ctx: *mut LicenseManagerCtx,
            out_buffer: *mut c_char,
            buffer_size: c_int,
        ) -> LM_Result,
    >,

    /// `LM_ReleaseAll(ctx) -> LM_Result`
    pub release_all:
        libloading::Symbol<'static, unsafe extern "C" fn(ctx: *mut LicenseManagerCtx) -> LM_Result>,

    /// Keep the loaded library alive
    _lib: Arc<libloading::Library>,
}

impl LicenseProxyApi {
    /// Load the license proxy DLL from the given path.
    ///
    /// # Safety
    ///
    /// This function loads a native shared library and resolves function
    /// symbols. The DLL must export the expected symbols with the correct
    /// calling convention and signatures.
    pub fn load(path: &str) -> Result<Self, libloading::Error> {
        let lib = Arc::new(unsafe { libloading::Library::new(path)? });

        /// Helper to load a symbol and transmute to 'static lifetime.
        ///
        /// # Safety
        ///
        /// The returned symbol is valid as long as the `Arc<Library>` is alive.
        /// We ensure this by storing `_lib` in the returned struct.
        unsafe fn load_sym<T>(
            lib: &libloading::Library,
            name: &[u8],
        ) -> Result<libloading::Symbol<'static, T>, libloading::Error> {
            let s = unsafe { lib.get::<T>(name) }?;
            Ok(unsafe { mem::transmute(s) })
        }

        unsafe {
            Ok(Self {
                create: load_sym(&lib, b"LM_Create")?,
                destroy: load_sym(&lib, b"LM_Destroy")?,
                initialize: load_sym(&lib, b"LM_Initialize")?,
                check_out: load_sym(&lib, b"LM_CheckOut")?,
                check_in: load_sym(&lib, b"LM_CheckIn")?,
                get_token_pool: load_sym(&lib, b"LM_GetTokenPool")?,
                is_star_license: load_sym(&lib, b"LM_IsStarLicense")?,
                get_server_info: load_sym(&lib, b"LM_GetServerInfo")?,
                release_all: load_sym(&lib, b"LM_ReleaseAll")?,
                _lib: lib,
            })
        }
    }
}
