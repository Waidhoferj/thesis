use lib0::decoding::{Cursor, Read};
use serde_json::Value as JSON;
pub struct Decoder<'a>(Cursor<'a>);

impl<'a> Read for Decoder<'a> {
    fn read_u8(&mut self) -> u8 {
        self.0.read_u8()
    }

    fn read(&mut self, len: usize) -> &[u8] {
        self.0.read(len)
    }
}

impl<'a> Decoder<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        let cursor = Cursor::new(buf);
        Decoder(cursor)
    }
    pub fn read_json(&mut self) -> JSON {
        // TODO Send length in header
        let json_string = self.read_string();
        json_string.into()
    }
}
