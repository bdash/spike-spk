use std::path::PathBuf;

use clap::Parser as _;

#[derive(Debug, clap::Parser)]
#[clap(about = "Verify the contents of a Spike2 SPK file")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

trait Command {
    fn run(&self) -> Result<(), Box<dyn std::error::Error>>;
}

#[derive(Debug, clap::Subcommand)]
enum Commands {
    /// Verify the contents of a SPK file.
    Verify(VerifyCommand),
    /// Extract the contents of a SPK file.
    Extract(ExtractCommand),
}

impl Command for Commands {
    fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Commands::Verify(cmd) => cmd.run(),
            Commands::Extract(cmd) => cmd.run(),
        }
    }
}

#[derive(Debug, clap::Args)]
struct VerifyCommand {
    /// The path to the SPK file to verify.
    path: PathBuf,
}

impl Command for VerifyCommand {
    fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = spike2_spk::SPKFile::open(&self.path)?;
        spike2_spk::verify::verify(&mut file)?;
        Ok(())
    }
}

#[derive(Debug, clap::Args)]
struct ExtractCommand {
    /// The path to the SPK file to extract.
    path: PathBuf,

    /// The directory to extract the files to.
    /// If not specified, the files will be extracted to the directory containing the SPK file.
    #[arg(short, long, name = "DIR")]
    output: Option<PathBuf>,
}

impl Command for ExtractCommand {
    fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        Err("Extracting is not yet implemented")?
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    args.command.run()
}
