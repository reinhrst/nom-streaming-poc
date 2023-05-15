use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, Write,};
use tempfile;

use nom::{
    bytes,
    Err,
    IResult,
};

/// Since we want a file to parse, let's start by creating something
fn create_rewound_file_with_100_times_0x00_to_0xff() -> Result<File, Box<dyn Error>> {
    let mut file = tempfile::tempfile()?;
    let range: Vec<u8> = (0u8..=0xff).collect();
    for _ in 0usize..100 {
        file.write(&range)?;
    }
    file.rewind()?;
    return Ok(file);
}

/// Some structure with first 8 bytes, then one byte, then another 8 bytes
/// This stands in for the thing that we actually want to get from our parser
struct ComplexStructure {
    part1: [u8; 8],
    part2: u8,
    part3: [u8; 8],
}

fn parse_complex_structure(input: &[u8]) -> IResult<&[u8], ComplexStructure> {
    let mut complex_structure = ComplexStructure {
        part1: [0; 8],
        part2: 0,
        part3: [0; 8],
    };
    let (input, part1) = bytes::streaming::take(8usize)(input)?;
    let (input, part2_slice) = bytes::streaming::take(1usize)(input)?;
    let (input, part3) = bytes::streaming::take(8usize)(input)?;

    complex_structure.part1.clone_from_slice(part1);
    complex_structure.part2 = part2_slice[0];
    complex_structure.part3.clone_from_slice(part3);
    return Ok((input, complex_structure));
}



/// We will read the file in chunks of this size
const CHUNK_SIZE: usize = 32;
struct FileIterator {
    file: File
}

impl Iterator for FileIterator {
    type Item = Vec<u8>;
    
    fn next(&mut self) -> Option<Self::Item> {
        let mut buffer: Vec<u8> = vec![0u8; CHUNK_SIZE];
        let len = self.file.read(&mut buffer).expect("Cannot read file");
        if len == 0 {
            // For now assuming EOF; probably in production code you might want to do something
            // else
            None
        } else {
            buffer.truncate(len);
            Some(buffer)
        }
    }
}

fn read_more_data_from_iterator(iterator: &mut dyn Iterator<Item=Vec<u8>>, unparsed_data: &mut Vec<u8>) -> Result<(), Box<dyn Error>> {
    // reading directly from file would lead to one less data-copy operation, but this abstraction
    // is clearer.
    if let Some(new_data) = iterator.next() {
        unparsed_data.extend(&new_data);
        Ok(())
    } else {
        Err("EOF")?
    }
}

struct ComplexStructresParser {
    input_iterator: Box<dyn Iterator<Item=Vec<u8>>>,
    unparsed_data: Vec<u8>
}


impl ComplexStructresParser {
    pub fn new(input_iterator: Box<dyn Iterator<Item=Vec<u8>>>) -> ComplexStructresParser {
        return Self {
            input_iterator,
            unparsed_data: vec![]
        }
    }
}

impl Iterator for ComplexStructresParser {
    type Item = ComplexStructure;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match parse_complex_structure(&self.unparsed_data) {
                Ok((new_unparsed_data, return_value)) => {
                    self.unparsed_data = new_unparsed_data.to_vec();
                    return Some(return_value);
                }
                Err(Err::Incomplete(_)) => {
                    println!("More data needed");
                    match read_more_data_from_iterator(&mut self.input_iterator, &mut self.unparsed_data) {
                        Ok(()) => {}
                        Err(_) => {
                            if self.unparsed_data.len() == 0 {
                                println!("Done");
                                return None;
                            } else {
                                println!("There are {} bytes remaining", self.unparsed_data.len() );
                                return None;
                            }
                        }
                    }
                }
                Err(e) => {
                    panic!("Parse error: {}", e);
                }
            };
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let file = create_rewound_file_with_100_times_0x00_to_0xff()?;
    let complex_structure_parser = ComplexStructresParser::new(Box::new(FileIterator { file }));
    for complex_structure in complex_structure_parser {
        println!("Found structure with part2: {}", complex_structure.part2);
    }
    Ok(())
}
