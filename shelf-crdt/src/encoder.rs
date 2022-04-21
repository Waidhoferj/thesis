use lib0::encoding::Write;
use serde_json::Value as JSON;

pub struct Encoder(Vec<u8>);

impl Write for Encoder {
    fn write_u8(&mut self, value: u8) {
        self.0.write_u8(value);
    }

    fn write(&mut self, buf: &[u8]) {
        self.0.write_buf(buf);
    }
}

impl Encoder {
    pub fn new() -> Self {
        Encoder(Vec::new())
    }
    pub fn write_json(&mut self, val: JSON) {
        let json_string = val.to_string();
        self.write(&json_string.as_bytes());
    }
}

impl From<Encoder> for Vec<u8> {
    fn from(encoder: Encoder) -> Self {
        encoder.0
    }
}
