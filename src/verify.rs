use anyhow::Context as _;
use hmac::{self, Mac as _};
use md5::Digest;
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};
use sha1;

use crate::spk;

#[derive(Debug)]
struct VerificationResult {
    md5: bool,
    hmac: bool,
}

fn verify_one_file(
    file: &spk::SPKFile,
    file_info: &spk::FileInfo,
) -> anyhow::Result<VerificationResult> {
    let contents = file.read(file_info)?;

    let md5_digest = md5::Md5::digest(&contents);
    let md5_result = md5_digest == file_info.md5.into();

    let mut sha1_hmac = hmac::Hmac::<sha1::Sha1>::new_from_slice(spk::HMAC_KEY)?;
    sha1_hmac.update(&contents);
    let sha1_hmac_digest = sha1_hmac.finalize().into_bytes();
    let sha1_hmac_result = sha1_hmac_digest == file_info.hmac.into();

    Ok(VerificationResult {
        md5: md5_result,
        hmac: sha1_hmac_result,
    })
}

pub(crate) fn verify_all(file: &spk::SPKFile) -> anyhow::Result<()> {
    // Verify files from all packages in parallel, collecting only the failures.
    let failures = file
        .packages
        .par_iter()
        .map(|package| {
            package
                .files
                .par_iter()
                .map(|file_info| -> anyhow::Result<_> {
                    let result = verify_one_file(file, file_info).with_context(|| {
                        format!(
                            "Error attempting to verify file {} in package {}",
                            file_info.name, package.name
                        )
                    })?;
                    Ok((file_info, result.md5 && result.hmac))
                })
        })
        .flatten()
        .filter(|result| match result {
            Ok((_, result)) => !result,
            Err(_) => true,
        })
        .collect::<Result<Vec<_>, _>>()?;

    if failures.is_empty() {
        return Ok(());
    }

    anyhow::bail!(
        "Some files failed verification: {}",
        failures
            .iter()
            .map(|(file_info, _)| file_info.name.clone())
            .collect::<Vec<_>>()
            .join(", ")
    );
}

fn check(value: bool) -> &'static str {
    if value { "✔" } else { "✗" }
}

pub fn verify(file: &mut spk::SPKFile) -> anyhow::Result<()> {
    for (i, package) in file.packages.iter().enumerate() {
        if i > 0 {
            println!("\n");
        }

        println!("Package: {}", package.name);
        println!(
            "Version: {}.{}.{}",
            package.version.0, package.version.1, package.version.2
        );

        let mut results = package
            .files
            .par_iter()
            .map(|file_info| -> anyhow::Result<_> {
                Ok((file_info, verify_one_file(file, file_info)?))
            })
            .collect::<Result<Vec<_>, _>>()?;

        results.sort_by(|a, b| a.0.name.cmp(&b.0.name));

        for (file_info, result) in results {
            println!(
                "{:165} mode={:o} size={:11}  md5: {}  hmac: {}  ",
                format!("{}{}", package.type_.path_prefix(), file_info.name),
                file_info.mode,
                file_info.size,
                check(result.md5),
                check(result.hmac)
            );
        }
    }

    Ok(())
}
