use std::{io::Write as _, path::Path};

use anyhow::Context as _;

use crate::{spk, verify};

pub fn extract(file: &mut spk::SPKFile, to: &Path) -> anyhow::Result<()> {
    match std::fs::remove_dir_all(to) {
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        err @ Err(_) => err.with_context(|| {
            format!(
                "Failed to remove directory prior to extraction {}",
                to.display()
            )
        })?,
    }

    print!("Verifying contents of file...");
    std::io::stdout().flush()?;
    verify::verify_all(file)?;
    println!(" done!");

    for package in file.packages.iter() {
        println!("\n");

        let package_path = to.join(&package.name);
        println!(
            "Extracting package {} to {}",
            package.name,
            package_path.display()
        );

        for file_info in &package.files {
            if file_info.name.starts_with("/") {
                anyhow::bail!(
                    "Refusing to extract file whose path is absolute: {}",
                    file_info.name
                );
            }

            println!("   {}", file_info.name);
            let output_path = package_path.join(&file_info.name);
            let parent = output_path.parent().ok_or_else(|| {
                anyhow::anyhow!(
                    "Failed to get parent directory for {}",
                    output_path.display()
                )
            })?;

            std::fs::create_dir_all(parent)?;

            std::fs::write(&output_path, file.read(file_info)?)?;
            std::fs::set_permissions(
                &output_path,
                std::os::unix::fs::PermissionsExt::from_mode(file_info.mode as u32),
            )?;
        }
    }

    Ok(())
}
