//! Extension → conversion mapping for CAD files.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Supported CAD file formats
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CadFormat {
    /// AutoCAD DWG
    Dwg,
    /// AutoCAD DXF (Drawing Exchange Format)
    Dxf,
    /// STEP (Standard for the Exchange of Product Data)
    Step,
    /// IGES (Initial Graphics Exchange Specification)
    Iges,
    /// STL (Stereolithography)
    Stl,
    /// OBJ (Wavefront)
    Obj,
    /// FBX (Filmbox)
    Fbx,
    /// 3DS (3D Studio)
    ThreeDs,
    /// Solidworks part
    Sldprt,
    /// Solidworks assembly
    Sldasm,
    /// CATIA
    CatPart,
    /// ProE/Creo
    Prt,
}

impl CadFormat {
    /// Determine CAD format from a file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "dwg" => Some(Self::Dwg),
            "dxf" => Some(Self::Dxf),
            "step" | "stp" => Some(Self::Step),
            "iges" | "igs" => Some(Self::Iges),
            "stl" => Some(Self::Stl),
            "obj" => Some(Self::Obj),
            "fbx" => Some(Self::Fbx),
            "3ds" => Some(Self::ThreeDs),
            "sldprt" => Some(Self::Sldprt),
            "sldasm" => Some(Self::Sldasm),
            "catpart" => Some(Self::CatPart),
            "prt" => Some(Self::Prt),
            _ => None,
        }
    }

    /// Get the display name for this format
    pub fn display_name(&self) -> &str {
        match self {
            Self::Dwg => "AutoCAD DWG",
            Self::Dxf => "AutoCAD DXF",
            Self::Step => "STEP",
            Self::Iges => "IGES",
            Self::Stl => "STL",
            Self::Obj => "Wavefront OBJ",
            Self::Fbx => "FBX",
            Self::ThreeDs => "3D Studio",
            Self::Sldprt => "SolidWorks Part",
            Self::Sldasm => "SolidWorks Assembly",
            Self::CatPart => "CATIA Part",
            Self::Prt => "ProE/Creo Part",
        }
    }
}

impl std::fmt::Display for CadFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Target format for conversion output
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConversionTarget {
    /// PDF document
    Pdf,
    /// SVG vector image
    Svg,
    /// PNG raster image
    Png,
    /// JPEG raster image
    Jpg,
    /// Thumbnail (small PNG)
    Thumbnail,
    /// 3D viewer format (glTF)
    Gltf,
}

impl ConversionTarget {
    /// Get the file extension for this target
    pub fn extension(&self) -> &str {
        match self {
            Self::Pdf => "pdf",
            Self::Svg => "svg",
            Self::Png => "png",
            Self::Jpg => "jpg",
            Self::Thumbnail => "thumb.png",
            Self::Gltf => "gltf",
        }
    }

    /// Get the MIME type for this target
    pub fn mime_type(&self) -> &str {
        match self {
            Self::Pdf => "application/pdf",
            Self::Svg => "image/svg+xml",
            Self::Png => "image/png",
            Self::Jpg => "image/jpeg",
            Self::Thumbnail => "image/png",
            Self::Gltf => "model/gltf+json",
        }
    }
}

impl std::fmt::Display for ConversionTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.extension())
    }
}

/// A mapping entry defining how to convert a CAD format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionMappingEntry {
    /// The source CAD format
    pub source: CadFormat,
    /// The target output formats
    pub targets: Vec<ConversionTarget>,
    /// The command to execute for conversion
    pub command: String,
    /// Command arguments template. Placeholders: {input}, {output}, {format}
    pub args_template: Vec<String>,
    /// Timeout in seconds for the conversion
    pub timeout_seconds: u64,
    /// Whether this mapping is enabled
    pub enabled: bool,
}

/// Registry of all conversion mappings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionMapping {
    /// All registered mappings
    mappings: HashMap<String, ConversionMappingEntry>,
}

impl ConversionMapping {
    /// Create a new empty mapping registry
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
        }
    }

    /// Create default mappings using common CAD conversion tools
    pub fn default_mappings() -> Self {
        let mut mappings = HashMap::new();

        // DWG → PDF, PNG, SVG using ODA File Converter or LibreCAD
        mappings.insert(
            "dwg".to_string(),
            ConversionMappingEntry {
                source: CadFormat::Dwg,
                targets: vec![
                    ConversionTarget::Pdf,
                    ConversionTarget::Png,
                    ConversionTarget::Thumbnail,
                ],
                command: "ODAFileConverter".to_string(),
                args_template: vec![
                    "{input_dir}".to_string(),
                    "{output_dir}".to_string(),
                    "0".to_string(),
                    "1".to_string(),
                    "0".to_string(),
                    "1".to_string(),
                ],
                timeout_seconds: 120,
                enabled: true,
            },
        );

        // DXF → PDF, PNG, SVG
        mappings.insert(
            "dxf".to_string(),
            ConversionMappingEntry {
                source: CadFormat::Dxf,
                targets: vec![
                    ConversionTarget::Pdf,
                    ConversionTarget::Png,
                    ConversionTarget::Svg,
                    ConversionTarget::Thumbnail,
                ],
                command: "libreoffice".to_string(),
                args_template: vec![
                    "--headless".to_string(),
                    "--convert-to".to_string(),
                    "{format}".to_string(),
                    "--outdir".to_string(),
                    "{output_dir}".to_string(),
                    "{input}".to_string(),
                ],
                timeout_seconds: 120,
                enabled: true,
            },
        );

        // STEP → PNG, glTF
        mappings.insert(
            "step".to_string(),
            ConversionMappingEntry {
                source: CadFormat::Step,
                targets: vec![
                    ConversionTarget::Png,
                    ConversionTarget::Thumbnail,
                    ConversionTarget::Gltf,
                ],
                command: "freecad-cmd".to_string(),
                args_template: vec![
                    "--run".to_string(),
                    "convert_step.py".to_string(),
                    "--input".to_string(),
                    "{input}".to_string(),
                    "--output".to_string(),
                    "{output}".to_string(),
                    "--format".to_string(),
                    "{format}".to_string(),
                ],
                timeout_seconds: 180,
                enabled: true,
            },
        );

        // STL → PNG, glTF
        mappings.insert(
            "stl".to_string(),
            ConversionMappingEntry {
                source: CadFormat::Stl,
                targets: vec![
                    ConversionTarget::Png,
                    ConversionTarget::Thumbnail,
                    ConversionTarget::Gltf,
                ],
                command: "meshlab-server".to_string(),
                args_template: vec![
                    "-i".to_string(),
                    "{input}".to_string(),
                    "-o".to_string(),
                    "{output}".to_string(),
                ],
                timeout_seconds: 120,
                enabled: true,
            },
        );

        Self { mappings }
    }

    /// Look up the mapping for a file extension
    pub fn get_mapping(&self, extension: &str) -> Option<&ConversionMappingEntry> {
        let ext = extension.to_lowercase();
        self.mappings.get(&ext).filter(|m| m.enabled)
    }

    /// Check if a file extension is a known CAD format
    pub fn is_cad_file(&self, extension: &str) -> bool {
        let ext = extension.to_lowercase();
        self.mappings.contains_key(&ext)
    }

    /// Get all supported extensions
    pub fn supported_extensions(&self) -> Vec<String> {
        self.mappings
            .iter()
            .filter(|(_, m)| m.enabled)
            .map(|(ext, _)| ext.clone())
            .collect()
    }

    /// Register a custom mapping
    pub fn register(&mut self, extension: String, entry: ConversionMappingEntry) {
        self.mappings.insert(extension, entry);
    }

    /// Enable or disable a mapping
    pub fn set_enabled(&mut self, extension: &str, enabled: bool) -> bool {
        if let Some(mapping) = self.mappings.get_mut(extension) {
            mapping.enabled = enabled;
            true
        } else {
            false
        }
    }
}

impl Default for ConversionMapping {
    fn default() -> Self {
        Self::default_mappings()
    }
}
