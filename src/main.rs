// MIT License
//
// Copyright (c) 2021-2024 Brenden Davidson
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use clap::{ArgAction, Parser};
use std::fs::File;
use std::path::PathBuf;

mod bios;

/// Cross-platform BIOS file renaming tool for ASUS motherboards
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to BIOS file to operate on
    bios_path: PathBuf,

    /// Target output directory for the renamed file
    #[arg(short, long)]
    out_dir: Option<PathBuf>,

    /// Copy the BIOS file instead of moving it
    #[arg(short, long, action = ArgAction::SetTrue, default_value = "false")]
    copy: bool,

    /// Do not show BIOS file details
    #[arg(long, action = ArgAction::SetTrue, default_value = "false")]
    hide_details: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let bios_path = cli.bios_path.canonicalize()?;

    let mut bios_file = File::open(&bios_path)?;

    // Check file validity
    let is_valid = bios::is_file_valid(&bios_file)?;

    if !is_valid {
        return Err(anyhow::Error::msg("INVALID PATH: provided path does not point to a file"));
    }

    let bios_info = bios::BiosInfo::from_file(&mut bios_file)?;
    // Close the file by dropping it
    drop(bios_file);


    // Handle the user setting a target directory
    let mut output_path = match cli.bios_path.parent() {
        Some(dir) => dir.to_owned(),
        None => {
            let mut out = PathBuf::new();
            out.push(".");
            out
        }
    };

    if let Some(dir) = cli.out_dir {
        output_path = dir;
    }

    // Print file info
    if !cli.hide_details {
        println!("\n{bios_info}\n");
    }

    // Rename source file
    output_path.push(bios_info.get_expected_name());
    println!("Output path: {}", &output_path.display());

    let should_copy = cli.copy;

    if should_copy {
        match std::fs::copy(&bios_path, &output_path) {
            Ok(_) => {
                println!("BIOS file copied to: {}", &output_path.display());
            }
            Err(why) => {
                let err_msg = format!("ERROR: Failed to copy file: {}", why);
                return Err(anyhow::Error::msg(err_msg));
            }
        };
    } else {
        // TODO: figure out how to handle when a user wishes to move the file to an external drive
        match std::fs::rename(&bios_path, &output_path) {
            Ok(_) => {
                println!("BIOS file moved to: {}", &output_path.display());
            }
            Err(why) => {
                let err_msg = format!("ERROR: Failed to move file: {}", why);
                return Err(anyhow::Error::msg(err_msg));
            }
        };
    }

    Ok(())
}
