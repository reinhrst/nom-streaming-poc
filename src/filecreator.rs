use std::error::Error;
use std::fs::File;
use std::io::{Seek, Write};
use tempfile;

pub fn create_rewound_file() -> Result<File, Box<dyn Error>> {
    let mut file = tempfile::tempfile()?;
    for i in 1u8..0x10 {
        let range: Vec<u8> = (0u8..=i).collect();
        file.write(&range)?;
        if i % 2 == 0 {
            file.write(&[0u8])?;
        }
    }
    file.rewind()?;
    return Ok(file);
}
