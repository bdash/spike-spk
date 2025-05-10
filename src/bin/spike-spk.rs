use std::{ffi::OsStr, path::PathBuf};

use clap::Parser as _;

/// Extract or verify a Stern Pinball software update package
///
/// Update files can be provided as the path to a single .spk file,
/// the path to a directory containing the split update files (.spk.OOX.00{1,2,...}),
/// or the path to the first of the spilt update files (.spk.OON.000).
#[derive(Debug, clap::Parser)]
#[clap(version, about)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

trait Command {
    fn run(&self) -> anyhow::Result<()>;
}

#[derive(Debug, clap::Subcommand)]
enum Commands {
    /// Verify the contents of a SPK file.
    Verify(VerifyCommand),
    /// Extract the contents of a SPK file.
    Extract(ExtractCommand),
}

impl Command for Commands {
    fn run(&self) -> anyhow::Result<()> {
        match self {
            Commands::Verify(cmd) => cmd.run(),
            Commands::Extract(cmd) => cmd.run(),
        }
    }
}

#[derive(Debug, clap::Args)]
struct VerifyCommand {
    /// The path to the SPK file to verify.
    ///
    /// The path can be the path to a single .spk file, the path to a directory
    /// containing the split update files (.spk.OOX.00{1,2,...}),
    /// or the path to the first of the spilt update files (.spk.OON.000).
    path: PathBuf,
}

impl Command for VerifyCommand {
    fn run(&self) -> anyhow::Result<()> {
        let mut file = spike_spk::SPKFile::open(&self.path)?;
        spike_spk::verify::verify(&mut file)
    }
}

#[derive(Debug, clap::Args)]
struct ExtractCommand {
    /// The path to the SPK file to extract.
    ///
    /// The path can be the path to a single .spk file, the path to a directory
    /// containing the split update files (.spk.OOX.00{1,2,...}),
    /// or the path to the first of the spilt update files (.spk.OON.000).
    path: PathBuf,

    /// The directory to extract the files to.
    ///
    /// If not specified, the files will be extracted to a directory alongside the SPK file.
    #[arg(short, long, name = "DIR")]
    output: Option<PathBuf>,
}

fn file_name_prefix(path: &PathBuf) -> Option<&OsStr> {
    let file_name = path.file_name()?.to_str()?;
    let i = match file_name[1..].find('.') {
        Some(i) => i + 1,
        None => return None,
    };
    Some(file_name[..i].as_ref())
}

impl Command for ExtractCommand {
    fn run(&self) -> anyhow::Result<()> {
        let path = std::path::absolute(&self.path)?;
        let mut file = spike_spk::SPKFile::open(&path)?;

        let prefix = file_name_prefix(&path).ok_or_else(|| {
            anyhow::anyhow!(
                "Could not determine file name prefix from path: {}",
                path.display()
            )
        })?;

        let output_directory = self
            .output
            .as_deref()
            .or_else(|| path.parent())
            .map(|p| p.join(prefix))
            .ok_or_else(|| anyhow::anyhow!("No output directory specified and default output directory could not be computed"))?;

        spike_spk::extract::extract(&mut file, &output_directory)
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    args.command.run()
}
