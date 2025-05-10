use std::io::Write;

use hmac::{self, Mac as _};
use md5::Digest;
use sha1;

use crate::spk;

pub fn verify(file: &mut spk::SPKFile) -> Result<(), Box<dyn std::error::Error>> {
    for (i, package) in file.packages.iter().enumerate() {
        if i > 0 {
            println!("\n");
        }

        println!("Package: {}", package.name);
        println!(
            "Version: {}.{}.{}",
            package.version.0, package.version.1, package.version.2
        );

        for file_info in &package.files {
            print!(
                "{:165} mode={:o} size={:11}  ",
                format!("{}{}", package.type_.path_prefix(), file_info.name),
                file_info.mode,
                file_info.size
            );
            std::io::stdout().flush()?;

            let file_contents = file.read(file_info)?;

            let md5_digest = md5::Md5::digest(&file_contents);
            if md5_digest == file_info.md5.into() {
                print!("md5: ✔  ");
            } else {
                print!("md5: ✗  ");
            }
            std::io::stdout().flush()?;

            let mut sha1_hmac = hmac::Hmac::<sha1::Sha1>::new_from_slice(spk::HMAC_KEY)?;
            sha1_hmac.update(&file_contents);
            let sha1_hmac_digest = sha1_hmac.finalize().into_bytes();
            if sha1_hmac_digest == file_info.hmac.into() {
                println!("hmac: ✔");
            } else {
                println!("hmac: ✗");
            }
        }
    }

    Ok(())
}
