use std::{io::Write as _, path::Path};

use anyhow::Context as _;

use crate::{
    fs::{FileSystem, WriteResult},
    spk, verify,
};

pub fn extract(file: &mut spk::SPKFile, to: &Path, fs: &mut dyn FileSystem) -> anyhow::Result<()> {
    match std::fs::remove_dir_all(to) {
        Ok(()) => {}
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

    for package in &file.packages {
        println!("\n");

        let package_path = to.join(&package.name);
        println!(
            "Extracting package {} to {}",
            package.name,
            package_path.display()
        );

        for file_info in &package.files {
            if file_info.name.starts_with('/') {
                anyhow::bail!(
                    "Refusing to extract file whose path is absolute: {}",
                    file_info.name
                );
            }

            let output_path = package_path.join(&file_info.name);
            let data = file.read(file_info)?;
            let result = fs.write_file(
                &output_path,
                &data,
                u32::from(file_info.mode),
                &file_info.hmac,
            )?;

            match result {
                WriteResult::Direct => println!("   {}", file_info.name),
                WriteResult::Cas { cached, .. } => {
                    let status = if cached { "(cached)" } else { "" };
                    println!("   {} {status}", file_info.name);
                }
            }
        }
    }

    Ok(())
}
