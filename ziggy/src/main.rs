// mod peermessage;

use fbencoding::Value;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

fn main() -> io::Result<()> {
    let path = Path::new("../torrent/seed-dir/simple-file.txt.torrent");
    println!("Path: {:?}", &path);
    let mut torrent = File::open(&path)?;
    let mut buffer = Vec::new();

    torrent.read_to_end(&mut buffer)?;

    let decoded = Value::decode(&buffer).unwrap();
    println!("Decoded Torrent: {}", &decoded);

    Ok(())
}
