use heif::HeicDecoder;

fn main() {
    let data = std::fs::read("./halfmoonbay.heic").unwrap();

    HeicDecoder::decode(&data).unwrap();
}
