//! Python script generation for Jupiter-Web batch mode.

use crate::error::ConversionError;
use crate::filesystem::FsUtils;
use crate::models::{ConversionInput, ConversionMode};

use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

/// Generates Jupiter Python scripts.
pub struct ScriptingEngine;

impl ScriptingEngine {
    /// Generate a Python script file for a conversion job.
    pub async fn generate_python_script(
        inputs: &[ConversionInput],
        output_path: &Path,
        mode: ConversionMode,
        primary_input: Option<&ConversionInput>,
        temp_dir: &Path,
    ) -> Result<PathBuf, ConversionError> {
        if inputs.is_empty() {
            return Err(ConversionError::NoInputs);
        }

        let script_name = FsUtils::generate_unique_filename("DirectConvert", "py");
        let script_path = temp_dir.join(script_name);

        let content = match mode {
            ConversionMode::Single => Self::generate_single_content(&inputs[0], output_path)?,
            ConversionMode::Assembly => {
                let primary = primary_input.ok_or(ConversionError::PrimaryNotSpecified)?;
                Self::generate_single_content(primary, output_path)?
            }
            ConversionMode::Combine => Self::generate_combine_content(inputs, output_path)?,
        };

        let mut file = tokio::fs::File::create(&script_path).await?;
        file.write_all(content.as_bytes()).await?;
        file.flush().await?;

        Ok(script_path)
    }

    /// Script for a single file: import + export.
    fn generate_single_content(
        input: &ConversionInput,
        output_path: &Path,
    ) -> Result<String, ConversionError> {
        Ok(format!(
            "{}\n{}",
            input.generate_import_command()?,
            input.generate_export_command(output_path)
        ))
    }

    /// Script for combining: all imports then one export.
    fn generate_combine_content(
        inputs: &[ConversionInput],
        output_path: &Path,
    ) -> Result<String, ConversionError> {
        let mut lines = Vec::with_capacity(inputs.len() + 1);
        for input in inputs {
            lines.push(input.generate_import_command()?);
        }
        // Use first input's file type for export mode determination
        lines.push(inputs[0].generate_export_command(output_path));
        Ok(lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::FileType;

    fn make_input(name: &str, ft: FileType) -> ConversionInput {
        ConversionInput {
            path: PathBuf::from(format!("/data/{}", name)),
            original_name: name.to_string(),
            file_type: ft,
        }
    }

    #[test]
    fn test_single_produces_two_lines() {
        let input = make_input("part.stp", FileType::Step);
        let content = ScriptingEngine::generate_single_content(&input, Path::new("/out/part.vtfx"))
            .expect("ok");
        assert_eq!(content.lines().count(), 2);
        assert!(content.contains("HoopsExchangeImport"));
        assert!(content.contains("ExportVTFxFile"));
    }

    #[test]
    fn test_combine_produces_n_plus_one_lines() {
        let inputs = vec![
            make_input("a.stp", FileType::Step),
            make_input("b.igs", FileType::Iges),
            make_input("c.dwg", FileType::AutoCad),
        ];
        let content =
            ScriptingEngine::generate_combine_content(&inputs, Path::new("/out/combo.vtfx"))
                .expect("ok");
        assert_eq!(content.lines().count(), 4);
    }

    #[tokio::test]
    async fn test_empty_inputs_error() {
        let temp = tempfile::tempdir().expect("tempdir");
        let result = ScriptingEngine::generate_python_script(
            &[],
            Path::new("/out.vtfx"),
            ConversionMode::Single,
            None,
            temp.path(),
        )
        .await;
        assert!(matches!(result, Err(ConversionError::NoInputs)));
    }

    #[tokio::test]
    async fn test_assembly_without_primary_error() {
        let temp = tempfile::tempdir().expect("tempdir");
        let inputs = vec![make_input("part.stp", FileType::Step)];
        let result = ScriptingEngine::generate_python_script(
            &inputs,
            Path::new("/out.vtfx"),
            ConversionMode::Assembly,
            None,
            temp.path(),
        )
        .await;
        assert!(matches!(result, Err(ConversionError::PrimaryNotSpecified)));
    }
}
