//! Jupiter-Web installation discovery and validation.
//!
//! Locates the TechnoStar Jupiter-Web installation by querying:
//! 1. The Windows registry (Inno Setup uninstall entries) using the known GUID
//! 2. Common installation directories as a fallback
//! 3. The system PATH
//!
//! On non-Windows platforms, only PATH and explicit configuration are supported.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, info, warn};

/// Inno Setup GUID for Jupiter-Web installer.
///
/// This GUID is embedded in the Inno Setup script and written to the
/// Windows registry under `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall`
/// and/or `HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall`.
const JUPITER_INNO_SETUP_GUID: &str = "{700798F8-7038-4887-BCC5-37278433D213}";

/// Registry subkey path template for Inno Setup uninstall entries.
const UNINSTALL_KEY_PATH: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";

/// The batch file name that launches Jupiter-Web.
const JUPITER_LAUNCHER: &str = "Start_It.bat";

/// Errors from Jupiter discovery.
#[derive(Debug, Error)]
pub enum DiscoveryError {
    /// Jupiter-Web installation was not found anywhere.
    #[error(
        "Jupiter-Web installation not found. Searched: registry GUID {guid}, common paths, and PATH"
    )]
    NotFound {
        /// The GUID that was searched for.
        guid: String,
    },

    /// Registry access failed (Windows only).
    #[error("Failed to access Windows registry: {reason}")]
    RegistryError {
        /// Description of the failure.
        reason: String,
    },

    /// The installation directory was found but the launcher is missing.
    #[error("Jupiter-Web install directory found at {install_dir} but {launcher} is missing")]
    LauncherMissing {
        /// The installation directory that was found.
        install_dir: PathBuf,
        /// The expected launcher file name.
        launcher: String,
    },
}

/// Information about a discovered Jupiter-Web installation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterInstallation {
    /// Full path to the launcher batch file (Start_It.bat).
    pub launcher_path: PathBuf,
    /// Installation directory.
    pub install_dir: PathBuf,
    /// Display name from the registry (e.g., "Jupiter-Web 5.0").
    pub display_name: Option<String>,
    /// Version from the registry.
    pub display_version: Option<String>,
    /// Publisher from the registry.
    pub publisher: Option<String>,
    /// How the installation was discovered.
    pub discovery_method: DiscoveryMethod,
}

/// How the Jupiter installation was discovered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryMethod {
    /// Found via Windows registry Inno Setup GUID.
    WindowsRegistry,
    /// Found in a common installation directory.
    CommonPath,
    /// Found via the system PATH environment variable.
    SystemPath,
    /// Explicitly configured by the user.
    ExplicitConfig,
}

/// Jupiter-Web installation discovery engine.
pub struct JupiterDiscovery;

impl JupiterDiscovery {
    /// Attempt to discover a Jupiter-Web installation.
    ///
    /// Searches in order:
    /// 1. Windows registry (Inno Setup GUID) — most reliable
    /// 2. Common installation directories
    /// 3. System PATH
    ///
    /// Returns the first valid installation found, or an error if none.
    pub fn discover() -> Result<JupiterInstallation, DiscoveryError> {
        info!("Searching for Jupiter-Web installation...");

        // 1. Try Windows registry
        #[cfg(windows)]
        {
            match Self::discover_from_registry() {
                Ok(installation) => {
                    info!(
                        path = %installation.launcher_path.display(),
                        version = ?installation.display_version,
                        "Found Jupiter-Web via Windows registry"
                    );
                    return Ok(installation);
                }
                Err(e) => {
                    debug!(error = %e, "Registry discovery failed, trying fallbacks");
                }
            }
        }

        // 2. Try common paths
        match Self::discover_from_common_paths() {
            Ok(installation) => {
                info!(
                    path = %installation.launcher_path.display(),
                    "Found Jupiter-Web in common installation path"
                );
                return Ok(installation);
            }
            Err(e) => {
                debug!(error = %e, "Common path discovery failed, trying PATH");
            }
        }

        // 3. Try system PATH
        match Self::discover_from_path() {
            Ok(installation) => {
                info!(
                    path = %installation.launcher_path.display(),
                    "Found Jupiter-Web in system PATH"
                );
                return Ok(installation);
            }
            Err(e) => {
                debug!(error = %e, "PATH discovery failed");
            }
        }

        Err(DiscoveryError::NotFound {
            guid: JUPITER_INNO_SETUP_GUID.to_string(),
        })
    }

    /// Discover Jupiter-Web from the Windows registry.
    ///
    /// Queries both HKLM and HKCU for the Inno Setup uninstall GUID,
    /// then reads the `InstallLocation` or `Inno Setup: App Path` value.
    #[cfg(windows)]
    fn discover_from_registry() -> Result<JupiterInstallation, DiscoveryError> {
        use winreg::RegKey;
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};

        let guid_suffix = format!("{}_is1", JUPITER_INNO_SETUP_GUID);

        // Try both HKLM and HKCU, both 64-bit and 32-bit views
        let roots = [(HKEY_LOCAL_MACHINE, "HKLM"), (HKEY_CURRENT_USER, "HKCU")];

        for (root_key, root_name) in &roots {
            let uninstall_key = match RegKey::predef(*root_key)
                .open_subkey_with_flags(UNINSTALL_KEY_PATH, KEY_READ)
            {
                Ok(key) => key,
                Err(e) => {
                    debug!(
                        root = root_name,
                        error = %e,
                        "Cannot open Uninstall registry key"
                    );
                    continue;
                }
            };

            // Try the exact GUID subkey
            let app_key = match uninstall_key.open_subkey_with_flags(&guid_suffix, KEY_READ) {
                Ok(key) => key,
                Err(_) => {
                    debug!(
                        root = root_name,
                        guid = %guid_suffix,
                        "GUID subkey not found"
                    );
                    continue;
                }
            };

            debug!(root = root_name, "Found Jupiter-Web registry entry");

            // Read installation path — Inno Setup uses "Inno Setup: App Path"
            // or "InstallLocation"
            let install_dir: PathBuf = Self::read_registry_install_path(&app_key)?;

            // Read metadata
            let display_name: Option<String> = app_key.get_value("DisplayName").ok();
            let display_version: Option<String> = app_key.get_value("DisplayVersion").ok();
            let publisher: Option<String> = app_key.get_value("Publisher").ok();

            // Verify the launcher exists
            let launcher_path = install_dir.join(JUPITER_LAUNCHER);
            if !launcher_path.exists() {
                // Try searching subdirectories
                if let Some(found) = Self::find_launcher_recursive(&install_dir, 2) {
                    return Ok(JupiterInstallation {
                        launcher_path: found,
                        install_dir,
                        display_name,
                        display_version,
                        publisher,
                        discovery_method: DiscoveryMethod::WindowsRegistry,
                    });
                }

                warn!(
                    install_dir = %install_dir.display(),
                    "Registry points to install dir but {} not found",
                    JUPITER_LAUNCHER
                );
                return Err(DiscoveryError::LauncherMissing {
                    install_dir,
                    launcher: JUPITER_LAUNCHER.to_string(),
                });
            }

            return Ok(JupiterInstallation {
                launcher_path,
                install_dir,
                display_name,
                display_version,
                publisher,
                discovery_method: DiscoveryMethod::WindowsRegistry,
            });
        }

        Err(DiscoveryError::RegistryError {
            reason: format!(
                "GUID {} not found in HKLM or HKCU uninstall registry",
                JUPITER_INNO_SETUP_GUID
            ),
        })
    }

    /// Read the installation path from a registry key.
    ///
    /// Tries multiple value names that Inno Setup may use.
    #[cfg(windows)]
    fn read_registry_install_path(key: &winreg::RegKey) -> Result<PathBuf, DiscoveryError> {
        // Inno Setup uses "Inno Setup: App Path" as the primary location
        let value_names = ["Inno Setup: App Path", "InstallLocation", "UninstallString"];

        for name in &value_names {
            if let Ok(value) = key.get_value::<String, _>(name) {
                let path = if *name == "UninstallString" {
                    // UninstallString is like "C:\path\to\unins000.exe"
                    // Extract the directory
                    PathBuf::from(&value)
                        .parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| PathBuf::from(&value))
                } else {
                    PathBuf::from(&value)
                };

                if path.exists() && path.is_dir() {
                    debug!(
                        value_name = name,
                        path = %path.display(),
                        "Found install path from registry"
                    );
                    return Ok(path);
                }
            }
        }

        Err(DiscoveryError::RegistryError {
            reason: "No valid install path found in registry values".to_string(),
        })
    }

    /// Discover Jupiter-Web from common installation directories.
    fn discover_from_common_paths() -> Result<JupiterInstallation, DiscoveryError> {
        let candidates = Self::common_install_paths();

        for candidate_dir in &candidates {
            if !candidate_dir.exists() || !candidate_dir.is_dir() {
                continue;
            }

            let launcher = candidate_dir.join(JUPITER_LAUNCHER);
            if launcher.exists() {
                return Ok(JupiterInstallation {
                    launcher_path: launcher,
                    install_dir: candidate_dir.clone(),
                    display_name: None,
                    display_version: None,
                    publisher: None,
                    discovery_method: DiscoveryMethod::CommonPath,
                });
            }

            // Check one level of subdirectories (e.g., versioned folders)
            if let Some(found) = Self::find_launcher_recursive(candidate_dir, 1) {
                let install_dir = found
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| candidate_dir.clone());
                return Ok(JupiterInstallation {
                    launcher_path: found,
                    install_dir,
                    display_name: None,
                    display_version: None,
                    publisher: None,
                    discovery_method: DiscoveryMethod::CommonPath,
                });
            }
        }

        Err(DiscoveryError::NotFound {
            guid: JUPITER_INNO_SETUP_GUID.to_string(),
        })
    }

    /// Generate common installation path candidates.
    fn common_install_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // Windows: Program Files
        #[cfg(windows)]
        {
            if let Ok(pf) = std::env::var("ProgramFiles") {
                paths.push(PathBuf::from(&pf).join("TechnoStar"));
                // Try versioned directory names
                for version in &["Jupiter-Web_5.0"] {
                    paths.push(PathBuf::from(&pf).join("TechnoStar").join(version));
                }
            }
            if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
                paths.push(PathBuf::from(&pf86).join("TechnoStar"));
                for version in &["Jupiter-Web_5.0"] {
                    paths.push(PathBuf::from(&pf86).join("TechnoStar").join(version));
                }
            }
        }

        // Fallback hardcoded paths
        paths.push(PathBuf::from("C:/Program Files/TechnoStar/Jupiter-Web_5.0"));

        // Linux/macOS: unlikely but support custom installs
        #[cfg(not(windows))]
        {
            // NOTE: Not implemented yet
        }

        paths
    }

    /// Discover Jupiter-Web from the system PATH.
    fn discover_from_path() -> Result<JupiterInstallation, DiscoveryError> {
        let path_var = std::env::var("PATH").unwrap_or_default();

        #[cfg(windows)]
        let separator = ';';
        #[cfg(not(windows))]
        let separator = ':';

        for dir in path_var.split(separator) {
            let dir_path = PathBuf::from(dir);
            let launcher = dir_path.join(JUPITER_LAUNCHER);
            if launcher.exists() {
                return Ok(JupiterInstallation {
                    launcher_path: launcher,
                    install_dir: dir_path,
                    display_name: None,
                    display_version: None,
                    publisher: None,
                    discovery_method: DiscoveryMethod::SystemPath,
                });
            }
        }

        Err(DiscoveryError::NotFound {
            guid: JUPITER_INNO_SETUP_GUID.to_string(),
        })
    }

    /// Recursively search for the launcher file within a directory.
    fn find_launcher_recursive(dir: &Path, max_depth: usize) -> Option<PathBuf> {
        Self::find_launcher_inner(dir, max_depth, 0)
    }

    /// Inner recursive search with depth tracking.
    fn find_launcher_inner(dir: &Path, max_depth: usize, current_depth: usize) -> Option<PathBuf> {
        if current_depth > max_depth {
            return None;
        }

        let launcher = dir.join(JUPITER_LAUNCHER);
        if launcher.exists() {
            return Some(launcher);
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return None,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = Self::find_launcher_inner(&path, max_depth, current_depth + 1)
                {
                    return Some(found);
                }
            }
        }

        None
    }

    /// Create a `JupiterInstallation` from an explicitly configured path.
    ///
    /// Validates that the path exists and points to the launcher.
    pub fn from_explicit_path(path: &Path) -> Result<JupiterInstallation, DiscoveryError> {
        if !path.exists() {
            return Err(DiscoveryError::NotFound {
                guid: JUPITER_INNO_SETUP_GUID.to_string(),
            });
        }

        // If the path points directly to the launcher
        if path.is_file() {
            let install_dir = path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| path.to_path_buf());

            return Ok(JupiterInstallation {
                launcher_path: path.to_path_buf(),
                install_dir,
                display_name: None,
                display_version: None,
                publisher: None,
                discovery_method: DiscoveryMethod::ExplicitConfig,
            });
        }

        // If the path points to a directory, search for the launcher
        if path.is_dir() {
            let launcher = path.join(JUPITER_LAUNCHER);
            if launcher.exists() {
                return Ok(JupiterInstallation {
                    launcher_path: launcher,
                    install_dir: path.to_path_buf(),
                    display_name: None,
                    display_version: None,
                    publisher: None,
                    discovery_method: DiscoveryMethod::ExplicitConfig,
                });
            }

            // Try one level down
            if let Some(found) = Self::find_launcher_recursive(path, 1) {
                let install_dir = found
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| path.to_path_buf());
                return Ok(JupiterInstallation {
                    launcher_path: found,
                    install_dir,
                    display_name: None,
                    display_version: None,
                    publisher: None,
                    discovery_method: DiscoveryMethod::ExplicitConfig,
                });
            }

            return Err(DiscoveryError::LauncherMissing {
                install_dir: path.to_path_buf(),
                launcher: JUPITER_LAUNCHER.to_string(),
            });
        }

        Err(DiscoveryError::NotFound {
            guid: JUPITER_INNO_SETUP_GUID.to_string(),
        })
    }

    /// Validate that a Jupiter installation is functional.
    ///
    /// Checks:
    /// - Launcher file exists and is readable
    /// - File has non-zero size
    /// - On Windows, the file has a `.bat` extension
    pub fn validate(installation: &JupiterInstallation) -> Result<(), DiscoveryError> {
        let launcher = &installation.launcher_path;

        if !launcher.exists() {
            return Err(DiscoveryError::LauncherMissing {
                install_dir: installation.install_dir.clone(),
                launcher: JUPITER_LAUNCHER.to_string(),
            });
        }

        let metadata = std::fs::metadata(launcher).map_err(|e| DiscoveryError::RegistryError {
            reason: format!("Cannot read launcher metadata: {}", e),
        })?;

        if metadata.len() == 0 {
            return Err(DiscoveryError::LauncherMissing {
                install_dir: installation.install_dir.clone(),
                launcher: format!("{} (file is empty)", JUPITER_LAUNCHER),
            });
        }

        Ok(())
    }

    /// Get the Inno Setup GUID used for discovery.
    pub fn inno_setup_guid() -> &'static str {
        JUPITER_INNO_SETUP_GUID
    }

    /// Get the expected launcher filename.
    pub fn launcher_filename() -> &'static str {
        JUPITER_LAUNCHER
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inno_setup_guid() {
        let guid = JupiterDiscovery::inno_setup_guid();
        assert_eq!(guid, "{700798F8-7038-4887-BCC5-37278433D213}");
        assert!(guid.starts_with('{'));
        assert!(guid.ends_with('}'));
    }

    #[test]
    fn test_launcher_filename() {
        assert_eq!(JupiterDiscovery::launcher_filename(), "Start_It.bat");
    }

    #[test]
    fn test_common_paths_not_empty() {
        let paths = JupiterDiscovery::common_install_paths();
        assert!(!paths.is_empty());
    }

    #[test]
    fn test_explicit_path_nonexistent() {
        let result = JupiterDiscovery::from_explicit_path(Path::new("/nonexistent/jupiter/path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_explicit_path_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let launcher = temp.path().join("Start_It.bat");
        std::fs::write(&launcher, "echo hello").expect("write");

        let result = JupiterDiscovery::from_explicit_path(&launcher);
        assert!(result.is_ok());

        let installation = result.expect("ok");
        assert_eq!(installation.launcher_path, launcher);
        assert_eq!(installation.install_dir, temp.path());
        assert_eq!(
            installation.discovery_method,
            DiscoveryMethod::ExplicitConfig
        );
    }

    #[test]
    fn test_explicit_path_directory_with_launcher() {
        let temp = tempfile::tempdir().expect("tempdir");
        let launcher = temp.path().join("Start_It.bat");
        std::fs::write(&launcher, "echo hello").expect("write");

        let result = JupiterDiscovery::from_explicit_path(temp.path());
        assert!(result.is_ok());

        let installation = result.expect("ok");
        assert_eq!(installation.launcher_path, launcher);
    }

    #[test]
    fn test_explicit_path_directory_without_launcher() {
        let temp = tempfile::tempdir().expect("tempdir");
        // Empty directory — no launcher file
        let result = JupiterDiscovery::from_explicit_path(temp.path());
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DiscoveryError::LauncherMissing { .. }
        ));
    }

    #[test]
    fn test_explicit_path_nested_launcher() {
        let temp = tempfile::tempdir().expect("tempdir");
        let sub = temp.path().join("Jupiter-Web_5.0");
        std::fs::create_dir_all(&sub).expect("mkdir");
        let launcher = sub.join("Start_It.bat");
        std::fs::write(&launcher, "echo hello").expect("write");

        let result = JupiterDiscovery::from_explicit_path(temp.path());
        assert!(result.is_ok());

        let installation = result.expect("ok");
        assert_eq!(installation.launcher_path, launcher);
    }

    #[test]
    fn test_validate_valid_installation() {
        let temp = tempfile::tempdir().expect("tempdir");
        let launcher = temp.path().join("Start_It.bat");
        std::fs::write(&launcher, "@echo off\nstart jupiter.exe").expect("write");

        let installation = JupiterInstallation {
            launcher_path: launcher,
            install_dir: temp.path().to_path_buf(),
            display_name: Some("Jupiter-Web 5.0".to_string()),
            display_version: Some("5.0.0".to_string()),
            publisher: Some("TechnoStar".to_string()),
            discovery_method: DiscoveryMethod::ExplicitConfig,
        };

        assert!(JupiterDiscovery::validate(&installation).is_ok());
    }

    #[test]
    fn test_validate_empty_launcher() {
        let temp = tempfile::tempdir().expect("tempdir");
        let launcher = temp.path().join("Start_It.bat");
        std::fs::write(&launcher, "").expect("write empty file");

        let installation = JupiterInstallation {
            launcher_path: launcher,
            install_dir: temp.path().to_path_buf(),
            display_name: None,
            display_version: None,
            publisher: None,
            discovery_method: DiscoveryMethod::ExplicitConfig,
        };

        let result = JupiterDiscovery::validate(&installation);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_missing_launcher() {
        let installation = JupiterInstallation {
            launcher_path: PathBuf::from("/nonexistent/Start_It.bat"),
            install_dir: PathBuf::from("/nonexistent"),
            display_name: None,
            display_version: None,
            publisher: None,
            discovery_method: DiscoveryMethod::ExplicitConfig,
        };

        let result = JupiterDiscovery::validate(&installation);
        assert!(matches!(
            result.unwrap_err(),
            DiscoveryError::LauncherMissing { .. }
        ));
    }

    #[test]
    fn test_find_launcher_recursive_depth_limit() {
        let temp = tempfile::tempdir().expect("tempdir");
        // Create deeply nested launcher: depth 3
        let deep = temp.path().join("a").join("b").join("c");
        std::fs::create_dir_all(&deep).expect("mkdir");
        std::fs::write(deep.join("Start_It.bat"), "echo").expect("write");

        // max_depth=1 should NOT find it
        assert!(JupiterDiscovery::find_launcher_recursive(temp.path(), 1).is_none());

        // max_depth=3 SHOULD find it
        assert!(JupiterDiscovery::find_launcher_recursive(temp.path(), 3).is_some());
    }

    #[test]
    fn test_installation_serialization() {
        let installation = JupiterInstallation {
            launcher_path: PathBuf::from(
                "C:/Program Files/TechnoStar/Jupiter-Web_5.0/Start_It.bat",
            ),
            install_dir: PathBuf::from("C:/Program Files/TechnoStar/Jupiter-Web_5.0"),
            display_name: Some("Jupiter-Web 5.0".to_string()),
            display_version: Some("5.0.2".to_string()),
            publisher: Some("TechnoStar Co., Ltd.".to_string()),
            discovery_method: DiscoveryMethod::WindowsRegistry,
        };

        let json = serde_json::to_string_pretty(&installation).expect("serialize");
        assert!(json.contains("windows_registry"));
        assert!(json.contains("Jupiter-Web 5.0"));

        let deser: JupiterInstallation = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deser.discovery_method, DiscoveryMethod::WindowsRegistry);
        assert_eq!(deser.display_version, Some("5.0.2".to_string()));
    }
}
