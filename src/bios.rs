// MIT License
//
// Copyright (c) 2021-2023 Brenden Davidson
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

use chrono::NaiveDate;
use std::{
    fs::File,
    io::{BufReader, ErrorKind, Read},
};

const INFO_HEADER_LEN: usize = 9;
/// Byte array used to search for the start of the BIOS info block
const BIOS_INFO_HEADER: [u8; INFO_HEADER_LEN] =
    [0x24, 0x42, 0x4F, 0x4F, 0x54, 0x45, 0x46, 0x49, 0x24]; // "$BOOTEFI$"
/// Total size of the info block minus the header
const BIOS_INFO_SIZE: usize = 158;

/// All recent BIOS/UEFI files from ASUS are this exact size. Unused space in the file is filled with 0xFF values.
const EXPECTED_FILE_SIZE: u64 = 33558528;

/// Where the board name begins offset from the end of the info header
const BOARD_NAME_OFFSET: usize = 0x05;
/// Number of bytes reserved for the board name in the info block
const BOARD_NAME_LEN: usize = 60;

/// Where the brand name begins offset from the end of the info header
const BRAND_NAME_OFFSET: usize = 0x41;
/// Number of bytes reserved for the brand name in the info block
const BRAND_NAME_LEN: usize = 20;

/// Where the build date begins offset from the end of the info header
const DATE_OFFSET: usize = 0x56;
/// Number of bytes reserved for the build date in the info block
const DATE_LEN: usize = 10;

/// Where the build number begins offset from the end of the info header
const BUILD_NUMBER_OFFSET: usize = 0x61;
/// Number of bytes reserved for the build number in the info block
const BUILD_NUMBER_LEN: usize = 14;

/// Where the CAP file name begins offset from the end of the info header
const CAP_NAME_OFFSET: usize = 0x88;
/// Number of bytes reserved for the CAP file name in the info block
const CAP_NAME_LEN: usize = 12;

/// Information describing the BIOS/EFI file as read from its info block.
#[derive(Debug)]
pub struct BiosInfo {
    /// Name of target motherboard
    board_name: String,

    /// Brand of motherboard
    brand: String,

    /// Reported BIOS build date
    build_date: NaiveDate,

    /// Reported build number
    build_number: String,

    /// Filename the target motherboard expects this file to be named
    ///
    /// # Examples:
    ///     - "TGX570PW.CAP"
    ///     - "C8DH.CAP"
    expected_name: String,
}

/// Returns a new String where all characters after the first NULL have been removed
///
/// # Arguments
///
/// * `s` - input string
fn trim_after_null(s: &str) -> String {
    let mut trimmed = String::new();

    for ch in s.chars() {
        if ch == '\0' {
            break;
        }
        trimmed.push(ch);
    }

    trimmed
}

fn bytes_to_string(bytes: &Vec<u8>, read_pos: usize, read_len: usize) -> String {
    let range = read_pos..(read_pos + read_len);

    let chunk = &bytes[range];
    let tmp_str = String::from_utf8_lossy(chunk);

    trim_after_null(&tmp_str)
}

impl BiosInfo {
    /// Seeks through the input file until the `$BOOTEFI$` header is found
    ///
    /// # Arguments
    ///   - `reader` - reader to seek on
    ///
    /// # Returns
    /// An Option enum containing the current seek position in the BufReader if the block was found
    fn seek_to_bootefi_block(reader: &mut BufReader<&mut File>) -> Option<usize> {
        let mut mini_buf = [0u8; 1];
        let mut buf = [0u8; INFO_HEADER_LEN];

        let mut read_pos = 0;
        loop {
            // Check if the current byte is '$'
            match reader.read_exact(&mut mini_buf) {
                Ok(_) => {}
                Err(err) => match err.kind() {
                    ErrorKind::UnexpectedEof => {
                        return None;
                    }
                    _ => {}
                },
            }
            if mini_buf[0] != 0x24 {
                // Current byte is not '$'
                read_pos += 1;
                continue;
            }
            // Step back 1 byte to compare the entire 9-byte segment
            reader
                .seek_relative(-1)
                .expect("Failed to step reader back");

            // Reads 9 bytes into 'buf'. If EoF is encountered, break the loop and return 'None'
            match reader.read_exact(&mut buf) {
                Ok(_) => {}
                Err(err) => match err.kind() {
                    ErrorKind::UnexpectedEof => {
                        return None;
                    }
                    _ => {}
                },
            }

            read_pos += INFO_HEADER_LEN;

            // Determine if 'buf' matches "$BOOTEFI$"
            if buf == BIOS_INFO_HEADER {
                return Some(read_pos);
            }
        }
    }

    pub fn from_file(bios_file: &mut File) -> Result<Self, std::io::Error> {
        // Read in raw bytes of info struct
        let mut reader = BufReader::new(bios_file);
        match BiosInfo::seek_to_bootefi_block(&mut reader) {
            Some(pos) => pos,
            None => {
                return Err(std::io::Error::new(
                    ErrorKind::InvalidData,
                    "Missing $BOOTEFI$ header in file",
                ));
            }
        };

        let mut info_chunk = Vec::with_capacity(BIOS_INFO_SIZE);

        reader
            .take(BIOS_INFO_SIZE as u64)
            .read_to_end(&mut info_chunk)?;

        // Read each field out of the info chunk
        let board_name = bytes_to_string(&info_chunk, BOARD_NAME_OFFSET, BOARD_NAME_LEN);
        let brand = bytes_to_string(&info_chunk, BRAND_NAME_OFFSET, BRAND_NAME_LEN);

        let build_date = bytes_to_string(&info_chunk, DATE_OFFSET, DATE_LEN);
        let build_date =
            NaiveDate::parse_from_str(&build_date, "%m/%d/%Y").unwrap_or(NaiveDate::default());

        let build_number = bytes_to_string(&info_chunk, BUILD_NUMBER_OFFSET, BUILD_NUMBER_LEN);
        let cap_name = bytes_to_string(&info_chunk, CAP_NAME_OFFSET, CAP_NAME_LEN);

        Ok(BiosInfo {
            board_name,
            brand,
            build_date,
            build_number,
            expected_name: cap_name,
        })
    }

    pub fn get_expected_name(&self) -> &String {
        &self.expected_name
    }
}

/// Returns `true` if the provided file meets known requirements
///
/// # Details
///
/// Currently, only the expected size of the file can be checked. It is yet to be determined if
/// these files have something like an embedded checksum and where that might be.
///
/// # Arguments
///
/// * `bios_file` - file to verify
pub fn is_file_valid(bios_file: &File) -> Result<bool, std::io::Error> {
    let file_info = bios_file.metadata()?;

    return if !file_info.is_file() {
        Ok(false)
    } else {
        Ok(file_info.len() == EXPECTED_FILE_SIZE)
    };
}
