use heif::HeicDecoder;

fn main() {
    let data = std::fs::read("./halfmoonbay.heic").unwrap();

    let _ = HeicDecoder::decode(&data).unwrap();
}
