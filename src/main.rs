use heif::HeifReader;

fn main() {
    let data = std::fs::read("./halfmoonbay.heic").unwrap();

    let mut reader = HeifReader::new(&data);
    reader.read().unwrap();
}
