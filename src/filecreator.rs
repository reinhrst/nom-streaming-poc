use std::error::Error;
use std::fs::File;
use std::io::{Seek, Write};
use tempfile;

pub fn create_rewound_file(iterations: usize) -> Result<File, Box<dyn Error>> {
    let mut file = tempfile::tempfile()?;
    let mut single_iteration_data: Vec<u8> = vec![];

    for i in 1u8..0x10 {
        let mut range: Vec<u8> = (0u8..=i).collect();
        single_iteration_data.append(&mut range);
        if i % 2 == 0 {
            // extra 0 byte every two lines
            single_iteration_data.push(0);
        }
    }
    for _ in 0..iterations {
        file.write(&single_iteration_data)?;
    }
    file.rewind()?;
    return Ok(file);
}
