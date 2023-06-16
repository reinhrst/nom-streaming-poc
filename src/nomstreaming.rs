use std::error::Error;
use std::fs::File;
use std::io::Read;
use nom::{bytes, error, Err, IResult};

fn parse_until_null_byte(unparsed_data: &[u8]) -> IResult<&[u8], Vec<u8>> {
    let (unparsed_data, parse_result) =
        bytes::streaming::take_till::<_, _, error::Error<_>>(|b| b == 0x00)(unparsed_data)?;
    // make sure we always process _something_
    let minimal_null_bytes = if parse_result.len() == 0 { 1 } else { 0 };
    let (unparsed_data, _) = bytes::streaming::take_while_m_n(
        minimal_null_bytes, 1, |b| b == 0x00)(unparsed_data)?;
    Ok((unparsed_data, parse_result.to_vec()))
}

/// Since we want a file to parse, let's start by creating something

/// We will read the file in chunks of this size
const CHUNK_SIZE: usize = 8;
#[derive(Debug)]
pub struct FileIterator {
    file: File,
}

impl FileIterator {
    pub fn new(file: File) -> Self {
        Self {file}
    }
}

impl Iterator for FileIterator {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buffer: Vec<u8> = vec![0u8; CHUNK_SIZE];
        let len = self.file.read(&mut buffer).expect("Cannot read file");
        if len == 0 {
            None
        } else {
            buffer.truncate(len);
            Some(buffer)
        }
    }
}

pub struct NullDelimitedVectorParser {
    input_iterator: Box<dyn Iterator<Item = Vec<u8>>>,
    parsing_data: Vec<u8>, // store the current data-chunk here
    unparsed_data_offset: usize,
}

impl NullDelimitedVectorParser {
    pub fn new(input_iterator: Box<dyn Iterator<Item = Vec<u8>>>) -> NullDelimitedVectorParser {
        return Self {
            input_iterator,
            parsing_data: vec![],
            unparsed_data_offset: 0,
        };
    }

    pub fn get_slice(&self) -> &[u8] {
        &self.parsing_data[self.unparsed_data_offset..]
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

    fn read_more_data_from_source(&mut self) -> Result<(), Box<dyn Error>> {
        match self.input_iterator.next() {
            Some(new_data) => {
                self.parsing_data = [self.get_slice(), &new_data].concat().to_vec();
                self.unparsed_data_offset = 0;
                Ok(())
            }
            None => Err("EOF")?, // string error OK for POC but should be proper custom error in production
        }
    }
}

impl Iterator for NullDelimitedVectorParser {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match parse_until_null_byte(self.get_slice()) {
                Ok((new_unparsed_data, return_value)) => {
                    self.unparsed_data_offset = self.get_slice_offset(new_unparsed_data);
                    return Some(return_value.to_vec());
                }
                Err(Err::Incomplete(_)) => {
                    println!("More data needed");
                    match self.read_more_data_from_source() {
                        Ok(_) => continue,
                        Err(_) => {
                            if self.get_slice().len() == 0 {
                                println!("Done");
                            } else {
                                println!("There are {} bytes remaining", self.get_slice().len());
                            }
                            return None
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
