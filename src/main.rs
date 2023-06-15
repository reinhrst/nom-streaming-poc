use std::error::Error;
mod filecreator;
mod nomstreaming;


fn main() -> Result<(), Box<dyn Error>> {
    let file = filecreator::create_rewound_file()?;
    let parser = nomstreaming::NullDelimitedVectorParser::new(
        Box::new(nomstreaming::FileIterator::new (file)));
    for bs in parser {
        println!("Found {:x?}", bs)
    }
    Ok(())
}
