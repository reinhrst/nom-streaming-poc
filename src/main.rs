use memmap2::MmapOptions;
use std::error::Error;
use std::io::Read;
mod filecreator;
mod nomcomplete;
mod nomstreaming;

fn do_nom_complete() -> Result<(), Box<dyn Error>> {
    println!("======== NOM COMPLETE ======");
    let mut file = filecreator::create_rewound_file(1)?;
    let mut data: Vec<u8> = vec![];
    file.read_to_end(&mut data)?;
    println!("Loaded all data");
    let parser = nomcomplete::NullDelimitedVectorParser::new(&data);
    for bs in parser {
        println!("Found {:x?}", bs)
    }
    Ok(())
}

fn do_nom_streaming() -> Result<(), Box<dyn Error>> {
    println!("======== NOM STREAMING ======");
    let file = filecreator::create_rewound_file(1)?;
    let parser = nomstreaming::NullDelimitedVectorParser::new(Box::new(
        nomstreaming::FileIterator::new(file),
    ));
    for bs in parser {
        println!("Found {:x?}", bs)
    }
    Ok(())
}

fn do_nom_memmap() -> Result<(), Box<dyn Error>> {
    println!("======== NOM MEMMAP ======");
    const ITERATIONS: usize = 200_000;
    let file = filecreator::create_rewound_file(ITERATIONS)?;
    let data = unsafe { MmapOptions::new().map(&file)? };
    println!(
        "Loaded all data ({} MB) into vitual memory",
        data.len() / 1024 / 1024
    );
    let parser = nomcomplete::NullDelimitedVectorParser::new(&data);
    for (i, bs) in parser.enumerate() {
        if i % ITERATIONS == 0 {
            println!("Found {:x?} ({})", bs, i)
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    do_nom_complete()?;
    do_nom_streaming()?;
    do_nom_memmap()?;
    Ok(())
}
