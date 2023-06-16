use nom::{bytes, error, IResult};

fn parse_until_null_byte(unparsed_data: &[u8]) -> IResult<&[u8], Vec<u8>> {
    let (unparsed_data, parse_result) =
        bytes::complete::take_till::<_, _, error::Error<_>>(|b| b == 0x00)(unparsed_data)?;
    // make sure we always process _something_
    let minimal_null_bytes = if parse_result.len() == 0 { 1 } else { 0 };
    let (unparsed_data, _) = bytes::complete::take_while_m_n(
        minimal_null_bytes, 1, |b| b == 0x00)(unparsed_data)?;
    Ok((unparsed_data, parse_result.to_vec()))
}

pub struct NullDelimitedVectorParser<'a> {
    data: &'a [u8],
}

impl<'a> NullDelimitedVectorParser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
        }
    }
}

impl<'a> Iterator for NullDelimitedVectorParser<'a> {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.len() == 0 {
            return None;
        }
        let (new_unparsed_data, return_value) = parse_until_null_byte(self.data).expect("Parse error");
        self.data = new_unparsed_data;
        return Some(return_value.to_vec());
    }
}
