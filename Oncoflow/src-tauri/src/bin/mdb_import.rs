use std::{io::Read, path::PathBuf};

use clap::Parser;
use oncoflow_lib::migration::{run_extracted_import, sha256_file, ImportOptions};

#[derive(Debug, Parser)]
#[command(
    name = "mdb-import",
    about = "Read-only AllTable.mdb to local OncoFlow SQLite importer"
)]
struct Cli {
    #[arg(long, default_value = "legacy/AllTable.mdb")]
    source: PathBuf,
    #[arg(long, default_value = "migration/output/oncoflow.db")]
    output: PathBuf,
    #[arg(long, default_value = "migration/reports/migration_report.json")]
    json_report: PathBuf,
    #[arg(long, default_value = "migration/reports/migration_report.md")]
    markdown_report: PathBuf,
    #[arg(long)]
    replace: bool,
    #[arg(
        long,
        help = "Read the privacy-filtered ACE extraction stream from stdin"
    )]
    extracted_stdin: bool,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Migration failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    if !cli.extracted_stdin {
        return Err(
            "use migration/import_alltable.ps1; MDB extraction is accepted only over stdin".into(),
        );
    }
    let mut extracted_json_lines = String::new();
    std::io::stdin().read_to_string(&mut extracted_json_lines)?;
    if extracted_json_lines.trim().is_empty() {
        return Err("ACE extraction stream is empty".into());
    }
    let source_sha256 = sha256_file(&cli.source)?;
    let source_filename = cli
        .source
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("AllTable.mdb")
        .to_owned();
    let options = ImportOptions {
        destination: &cli.output,
        replace: cli.replace,
        source_filename: &source_filename,
        source_sha256: &source_sha256,
        json_report: &cli.json_report,
        markdown_report: &cli.markdown_report,
    };

    let report = run_extracted_import(&cli.source, &extracted_json_lines, &options)?;
    println!(
        "Migration complete: {} table report(s), integrity={}, foreign_key_violations={}, output={}",
        report.tables.len(),
        report.validation.integrity_check,
        report.validation.foreign_key_violation_count,
        cli.output.display()
    );
    Ok(())
}
