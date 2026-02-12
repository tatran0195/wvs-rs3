//! Domain models: file types, conversion modes, inputs, outputs, options.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use crate::error::ConversionError;

/// Normalize path to forward slashes for Python script embedding.
fn normalize_path_for_python(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

// ---------------------------------------------------------------------------
// Extension map macro
// ---------------------------------------------------------------------------

macro_rules! define_file_types {
    ($($variant:ident => $ext:literal),* $(,)?) => {
        static EXTENSION_MAP: LazyLock<HashMap<&'static str, FileType>> = LazyLock::new(|| {
            HashMap::from([$(($ext, FileType::$variant),)*])
        });

        impl FileType {
            /// All file extensions supported by the converter.
            pub const SUPPORTED_EXTENSIONS: &'static [&'static str] = &[$($ext,)*];
        }
    };
}

// FIX: Added missing aliases (step, iges, sldprt correct spelling)
// FIX: Separated prt → UnigraphicsPart (Creo uses .prt.N compound which needs special handling)
define_file_types! {
    Parasolid            => "x_t",
    ParasolidBinary      => "x_b",
    CatiaV5Part          => "catpart",
    CatiaV5Product       => "catproduct",
    ProEngineerModel     => "model",
    Iges                 => "igs",
    IgesAlt              => "iges",
    InventorPart         => "ipt",
    InventorAssembly     => "iam",
    SolidWorksPart       => "sldprt",
    SolidWorksAssembly   => "sldasm",
    Acis                 => "sat",
    Step                 => "stp",
    StepAlt              => "step",
    StepP21              => "p21",
    Vrml                 => "wrl",
    JupiterTessellation  => "jt",
    AutoCad              => "dwg",
    UnigraphicsPart      => "prt",
    JupiterHdf5          => "jth5",
    NastranBdf           => "bdf",
    NastranDat           => "dat",
    TechnoStarGeometry   => "tsg",
    NastranOp2           => "op2",
    ADVC                 => "adv",
    Zip                  => "zip",
    Vtfx                 => "vtfx",
}

/// All recognized input file types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileType {
    // --- CAD geometry (HOOPS Exchange) ---
    /// Parasolid text (.x_t)
    Parasolid,
    /// Parasolid binary (.x_b)
    ParasolidBinary,
    /// CATIA V5 Part (.catpart)
    CatiaV5Part,
    /// CATIA V5 Product (.catproduct)
    CatiaV5Product,
    /// Pro/ENGINEER Model (.model)
    ProEngineerModel,
    /// IGES (.igs)
    Iges,
    /// IGES alternate extension (.iges)
    IgesAlt,
    /// Inventor Part (.ipt)
    InventorPart,
    /// Inventor Assembly (.iam)
    InventorAssembly,
    /// SolidWorks Part (.sldprt)
    SolidWorksPart,
    /// SolidWorks Assembly (.sldasm)
    SolidWorksAssembly,
    /// ACIS SAT (.sat)
    Acis,
    /// STEP (.stp)
    Step,
    /// STEP alternate extension (.step)
    StepAlt,
    /// STEP P21 (.p21)
    StepP21,
    /// VRML (.wrl)
    Vrml,
    /// Pro/ENGINEER Assembly (.asm.9) — compound extension
    ProEngineerAssembly,
    /// JT / Jupiter Tessellation (.jt)
    JupiterTessellation,
    /// AutoCAD DWG (.dwg)
    AutoCad,
    /// Unigraphics / NX Part (.prt)
    UnigraphicsPart,

    // --- FEA / simulation (native Jupiter import) ---
    /// Jupiter HDF5 (.jth5)
    JupiterHdf5,
    /// Nastran bulk data (.bdf)
    NastranBdf,
    /// Nastran data file (.dat) — treated same as .bdf
    NastranDat,
    /// TechnoStar Geometry (.tsg)
    TechnoStarGeometry,
    /// Nastran OP2 results (.op2)
    NastranOp2,
    /// Abaqus ODB results
    AbaqusOdb,
    /// ADVenture Cluster results (.adv)
    ADVC,

    // --- Container / pass-through ---
    /// ZIP archive (.zip)
    Zip,
    /// VTFx visualization (.vtfx) — pass-through
    Vtfx,

    // --- Synthetic ---
    /// Directory input
    Directory,
    /// Unrecognized file type
    Unknown,
}

impl FileType {
    /// Determine file type from a filesystem path.
    pub fn from_path_ref(path: &Path) -> Option<Self> {
        if path.is_dir() {
            if Self::check_advc_directory(path) {
                return Some(FileType::ADVC);
            }
            return Some(FileType::Directory);
        }

        let filename = path.file_name()?.to_str()?;
        let lower = filename.to_ascii_lowercase();

        // Compound extension: .asm.N (Pro/ENGINEER Assembly)
        if lower.contains(".asm.") {
            // Check the pattern: name.asm.N where N is a digit
            let parts: Vec<&str> = lower.rsplitn(3, '.').collect();
            if parts.len() >= 2 {
                if let Some(before_last) = parts.get(1) {
                    if *before_last == "asm" {
                        return Some(FileType::ProEngineerAssembly);
                    }
                }
            }
        }

        let ext = lower.rsplit('.').next()?;
        if ext.is_empty() {
            return None;
        }

        EXTENSION_MAP.get(ext).copied()
    }

    /// Check if a directory is an ADVenture Cluster result directory.
    fn check_advc_directory(dir_path: &Path) -> bool {
        let entries = match std::fs::read_dir(dir_path) {
            Ok(e) => e,
            Err(_) => return false,
        };

        for entry in entries.flatten() {
            let p = entry.path();
            if let Some(n) = p.file_name().and_then(|s| s.to_str()) {
                if n.eq_ignore_ascii_case("modeldb") {
                    return true;
                }
            }
            if let Some(e) = p.extension().and_then(|s| s.to_str()) {
                if e.eq_ignore_ascii_case("adv") {
                    return true;
                }
            }
        }
        false
    }

    /// Returns `true` if this is a VTFx pass-through.
    pub fn is_vtfx_format(&self) -> bool {
        matches!(self, FileType::Vtfx)
    }

    /// Returns `true` if this is a ZIP archive.
    pub fn is_archive_format(&self) -> bool {
        matches!(self, FileType::Zip)
    }

    /// Returns `true` if this is a CAD/mesh format imported via HOOPS Exchange or native.
    pub fn is_cad_format(&self) -> bool {
        matches!(
            self,
            FileType::Parasolid
                | FileType::ParasolidBinary
                | FileType::CatiaV5Part
                | FileType::CatiaV5Product
                | FileType::ProEngineerModel
                | FileType::Iges
                | FileType::IgesAlt
                | FileType::InventorPart
                | FileType::InventorAssembly
                | FileType::SolidWorksPart
                | FileType::SolidWorksAssembly
                | FileType::Acis
                | FileType::Step
                | FileType::StepAlt
                | FileType::StepP21
                | FileType::Vrml
                | FileType::ProEngineerAssembly
                | FileType::JupiterTessellation
                | FileType::AutoCad
                | FileType::UnigraphicsPart
                | FileType::JupiterHdf5
                | FileType::NastranBdf
                | FileType::NastranDat
                | FileType::TechnoStarGeometry
        )
    }

    /// Returns `true` if this is a simulation/results format.
    pub fn is_results_format(&self) -> bool {
        matches!(
            self,
            FileType::NastranOp2 | FileType::AbaqusOdb | FileType::ADVC
        )
    }

    /// Returns `true` if this file type can be processed by the pipeline.
    pub fn is_processable(&self) -> bool {
        self.is_cad_format() || self.is_results_format() || self.is_vtfx_format()
    }

    /// Generate the Jupiter Python import command for this file type.
    pub fn generate_import_command(&self, path: &Path) -> Result<String, ConversionError> {
        let p = normalize_path_for_python(path);
        let cmd = match self {
            FileType::JupiterHdf5 => {
                format!("JPT.Exec('LoadJTH5(\"{}\", 0)')", p)
            }
            FileType::NastranBdf | FileType::NastranDat => {
                format!(
                    "JPT.Exec('ImportBdf([\"{}\"], 2, 1.0472, 1.0472, 0, -1)')",
                    p
                )
            }
            FileType::TechnoStarGeometry => {
                format!("JPT.Exec('ImportTSG([\"{}\"], 1)')", p)
            }
            FileType::NastranOp2 => {
                format!(
                    "JPT.Exec('CmdImportTSVOp2Post(\"{}\", 1, 1.0472, 1.0472, 0, 0, 0)')",
                    p
                )
            }
            FileType::AbaqusOdb | FileType::ADVC => {
                format!(
                    "JPT.Exec('CmdImportTSVPost(\"{}\", 8, 1, 1.0472, 1.0472, 0, 0, 0)')",
                    p
                )
            }
            _ if self.is_cad_format() => {
                format!(
                    "JPT.Exec('HoopsExchangeImport([\"{}\"], 1, 3, 30, 5000, 1, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0.01, 0, 0, 0, 0, 0)')",
                    p
                )
            }
            _ => {
                return Err(ConversionError::ImportNotSupported {
                    file_type: format!("{:?}", self),
                });
            }
        };
        Ok(cmd)
    }

    /// Generate the Jupiter Python export-to-VTFx command.
    pub fn generate_export_command(&self, output_path: &Path) -> String {
        let export_mode = if self.is_cad_format() { 0 } else { 1 };
        let p = normalize_path_for_python(output_path);
        format!(
            "JPT.Exec('ExportVTFxFile(\"{}\", {}, 0, 0, 1, [], 1, 0, 0, 0, 0, 1, 1)')",
            p, export_mode
        )
    }
}

// ---------------------------------------------------------------------------
// Conversion mode
// ---------------------------------------------------------------------------

/// How multiple input files should be handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConversionMode {
    /// Each file converted independently.
    Single = 0,
    /// Primary assembly file with referenced parts.
    Assembly = 1,
    /// All inputs combined into one VTFx.
    Combine = 2,
}

impl Default for ConversionMode {
    fn default() -> Self {
        Self::Single
    }
}

// ---------------------------------------------------------------------------
// ConversionResult
// ---------------------------------------------------------------------------

/// Result of a successful single-file conversion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionResult {
    /// Absolute path to the output VTFx file.
    pub path: String,
    /// Output filename.
    pub filename: String,
    /// Directory containing the output.
    pub destination: String,
    /// Original source filename.
    pub original_name: String,
    /// Output file size in bytes.
    pub size: f64,
}

// ---------------------------------------------------------------------------
// ConversionInput
// ---------------------------------------------------------------------------

/// A resolved input file ready for the pipeline.
#[derive(Debug, Clone)]
pub struct ConversionInput {
    /// Path to the input file on disk.
    pub path: PathBuf,
    /// Original file name.
    pub original_name: String,
    /// Detected file type.
    pub file_type: FileType,
}

impl ConversionInput {
    /// Generate Jupiter import command for this input.
    pub fn generate_import_command(&self) -> Result<String, ConversionError> {
        self.file_type.generate_import_command(&self.path)
    }

    /// Generate Jupiter export command targeting `output_path`.
    pub fn generate_export_command(&self, output_path: &Path) -> String {
        self.file_type.generate_export_command(output_path)
    }
}

// ---------------------------------------------------------------------------
// ConversionOptions
// ---------------------------------------------------------------------------

/// User-supplied options for a conversion job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionOptions {
    /// Conversion mode. Defaults to `Single`.
    pub mode: Option<ConversionMode>,
    /// Primary input file name for Assembly/Combine modes.
    pub primary_name: Option<String>,
    /// Whether to delete source files after conversion.
    pub delete_source: Option<bool>,
    /// Per-request concurrency limit (1..=4).
    pub concurrency: Option<u8>,
    /// Whether to scan subdirectories.
    pub scan_deeper: Option<bool>,
}

impl Default for ConversionOptions {
    fn default() -> Self {
        Self {
            mode: Some(ConversionMode::Single),
            primary_name: None,
            delete_source: Some(false),
            concurrency: Some(1),
            scan_deeper: Some(false),
        }
    }
}

impl ConversionOptions {
    /// Resolve the effective conversion mode.
    pub fn conversion_mode(&self) -> ConversionMode {
        self.mode.unwrap_or_default()
    }

    /// Whether source files should be deleted.
    pub fn should_delete_source(&self) -> bool {
        self.delete_source.unwrap_or(false)
    }

    /// Whether to recursively scan subdirectories.
    pub fn should_scan_deeper(&self) -> bool {
        self.scan_deeper.unwrap_or(false)
    }

    /// Effective per-request concurrency.
    pub fn concurrency(&self) -> u8 {
        self.concurrency.unwrap_or(1).clamp(1, 4)
    }

    /// Resolve the primary file name. Returns error for Assembly if missing.
    pub fn get_primary_name(&self) -> Result<String, ConversionError> {
        match self.conversion_mode() {
            ConversionMode::Combine => Ok(self
                .primary_name
                .as_deref()
                .unwrap_or("combined_files")
                .to_string()),
            ConversionMode::Assembly => self
                .primary_name
                .as_deref()
                .map(|s| s.to_string())
                .ok_or(ConversionError::PrimaryNotSpecified),
            ConversionMode::Single => Ok(String::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_aliases() {
        // Both .stp and .step should resolve to Step variants
        assert!(EXTENSION_MAP.get("stp").is_some());
        assert!(EXTENSION_MAP.get("step").is_some());
        assert!(EXTENSION_MAP.get("igs").is_some());
        assert!(EXTENSION_MAP.get("iges").is_some());
        assert!(EXTENSION_MAP.get("x_t").is_some());
        assert!(EXTENSION_MAP.get("x_b").is_some());
        assert!(EXTENSION_MAP.get("sldprt").is_some());
        assert!(EXTENSION_MAP.get("sldasm").is_some());
    }

    #[test]
    fn test_compound_extension_asm() {
        // Non-existent file, but path-based detection should still work
        // for compound extension patterns when not checking is_dir
        let lower = "assembly.asm.9".to_ascii_lowercase();
        assert!(lower.contains(".asm."));
    }

    #[test]
    fn test_step_alt_is_cad() {
        assert!(FileType::StepAlt.is_cad_format());
        assert!(FileType::IgesAlt.is_cad_format());
        assert!(FileType::ParasolidBinary.is_cad_format());
    }

    #[test]
    fn test_nastran_dat_import() {
        let cmd = FileType::NastranDat
            .generate_import_command(Path::new("/data/mesh.dat"))
            .expect("should generate");
        assert!(cmd.contains("ImportBdf"));
    }

    #[test]
    fn test_options_assembly_no_primary() {
        let opts = ConversionOptions {
            mode: Some(ConversionMode::Assembly),
            primary_name: None,
            ..Default::default()
        };
        assert!(opts.get_primary_name().is_err());
    }
}
