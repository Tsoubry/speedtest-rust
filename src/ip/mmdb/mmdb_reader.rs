use crate::ip::mmdb::mmdb_record::MMDBResult;
use log::warn;
use maxminddb::Reader;

pub struct MMDBReader {
    reader: Reader<Vec<u8>>,
}

impl MMDBReader {
    pub fn from(path: &str) -> Option<Self> {
        if let Ok(custom_reader) = maxminddb::Reader::open_readfile(path) {
            Some(MMDBReader {
                reader: custom_reader,
            })
        } else {
            None
        }
    }

    pub fn lookup(&mut self, address: &str) -> Option<MMDBResult> {
        match self.reader.lookup(address.parse().unwrap()) {
            Err(e) => {
                warn!("Geo IP database error: {}", e);
                None
            }
            Ok(o) => {
                if let Ok(Some(result)) = o.decode() {
                    Some(result)
                } else {
                    warn!("Failed to deserialise Geo IP data {:?}", o);
                    None
                }
            }
        }
    }
}
