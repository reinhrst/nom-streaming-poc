use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, Write,};
use tempfile;

use nom::{
    bytes,
    error,
    IResult,
    Err,
};

/// Since we want a file to parse, let's start by creating something
fn create_rewound_file() -> Result<File, Box<dyn Error>> {
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

/// We will read the file in chunks of this size
const CHUNK_SIZE: usize = 8;
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

fn read_more_data_from_iterator(iterator: &mut dyn Iterator<Item=Vec<u8>>) -> Result<Vec<u8>, Box<dyn Error>> {
    // reading directly from file would lead to one less data-copy operation, but this abstraction
    // is clearer.
    if let Some(new_data) = iterator.next() {
        Ok(new_data)
    } else {
        Err("EOF")?
    }
}

struct BytesParser {
    input_iterator: Box<dyn Iterator<Item=Vec<u8>>>,
    parsing_data: Vec<u8>,  // store the current data-chunk here
    unparsed_data_pointer: usize,
}

impl BytesParser {
    pub fn new(input_iterator: Box<dyn Iterator<Item=Vec<u8>>>) -> BytesParser {
        return Self {
            input_iterator,
            parsing_data: vec![],
            unparsed_data_pointer: 0,
        }
    }

    pub fn get_slice(&self) -> &[u8] {
        &self.parsing_data[self.unparsed_data_pointer..]
    }

    pub fn get_slice_offset(&self, slice: &[u8]) -> usize {
        let data_begin = self.parsing_data.as_ptr() as usize;
        let data_end = data_begin + self.parsing_data.len();
        let slice_begin = slice.as_ptr() as usize;
        let slice_end = slice_begin + slice.len();
        let slice_offset = slice_begin - data_begin;
        assert_eq!(data_end, slice_end);
        assert!(slice_offset <= self.parsing_data.len());
        slice_offset
    }
}

fn parse_until_null_byte(unparsed_data: &[u8]) -> IResult<&[u8], Vec<u8>> {
    let (unparsed_data, parse_result) = bytes::streaming::take_till::<_, _, error::Error<_>>(|b| b == 0x00)(unparsed_data)?;
    let (unparsed_data, _) = bytes::streaming::take_while_m_n(0, 1, |b| b == 0x00)(unparsed_data)?;
    Ok((unparsed_data, parse_result.to_vec()))

}

impl Iterator for BytesParser {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match parse_until_null_byte(self.get_slice()) {
                Ok((new_unparsed_data, return_value)) => {
                    self.unparsed_data_pointer = self.get_slice_offset(new_unparsed_data);
                    return Some(return_value.to_vec());
                }
                Err(Err::Incomplete(_)) => {
                    println!("More data needed");
                    match read_more_data_from_iterator(&mut self.input_iterator) {
                        Ok(new_data) => {
                            let unparsed_data = self.get_slice();
                            let mut new_parsing_data = Vec::with_capacity(unparsed_data.len() + new_data.len());
                            new_parsing_data.extend(unparsed_data);
                            new_parsing_data.extend(new_data);
                            self.parsing_data = new_parsing_data;
                            self.unparsed_data_pointer = 0;
                        }
                        Err(_) => {
                            if self.get_slice().len() == 0 {
                                println!("Done");
                                return None;
                            } else {
                                println!("There are {} bytes remaining", self.get_slice().len() );
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
    let file = create_rewound_file()?;
    for bs in BytesParser::new(Box::new(FileIterator { file })) {
        println!("Found {:x?}", bs)
    }
    Ok(())
}
