use std::io;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::fs::File;

pub enum Endianness {
    Big,
    Little,
}

enum InputData<'f> {
    File(&'f File),
    Input(&'f Input<'f>),
}

pub struct Input<'f> {
    input: InputData<'f>,
    offset: u64,
    limit: u64,
    cursor: u64,
}

impl<'f> Input<'f> {
    pub fn create(input_file: &'f File) -> Self {
        Input {
            input: InputData::File(input_file),
            offset: 0,
            limit: input_file.metadata().expect("metadata expected").len(),
            cursor: 0,
        }
    }

    pub fn read_u32(&mut self, bo: &Endianness) -> io::Result<u32> {
        // TODO overflow check
        if self.cursor + 4 >= self.limit {
            return Err(io::Error::new(
                ErrorKind::UnexpectedEof,
                format!("EOF: reading 4 bytes from {}, input length: {}", self.cursor, self.limit)));
        }
        let mut buf: [u8; 4] = [0; 4];
        match self.input {
            InputData::File(f) => f.read_exact(&mut buf)?,
            InputData::Input(_) => panic!("not implemented"),
        };
        self.cursor = self.cursor + 4;
        match bo {
            Endianness::Big => Ok(u32::from_be_bytes(buf)),
            Endianness::Little => Ok(u32::from_le_bytes(buf))
        }
    }

    pub fn read_u64(&mut self, bo: &Endianness) -> io::Result<u64> {
        // TODO overflow check
        if self.cursor + 8 >= self.limit {
            return Err(io::Error::new(
                ErrorKind::UnexpectedEof,
                format!("EOF: reading 8 bytes from {}, input length: {}", self.cursor, self.limit)));
        }
        let mut buf: [u8; 8] = [0; 8];
        match self.input {
            InputData::File(f) => f.read_exact(&mut buf)?,
            InputData::Input(_) => panic!("not implemented"),
        };
        self.cursor = self.cursor + 8;
        match bo {
            Endianness::Big => Ok(u64::from_be_bytes(buf)),
            Endianness::Little => Ok(u64::from_le_bytes(buf))
        }
    }

    pub fn read_string(&mut self, len: u64) -> io::Result<String> {
        // TODO overflow check
        if self.cursor + len >= self.limit {
            return Err(io::Error::new(
                ErrorKind::UnexpectedEof,
                format!("EOF: reading {} bytes from {}, input length: {}", len, self.cursor, self.limit)));
        }
        let mut take_input = match self.input {
            InputData::File(f) => f.take(len),
            InputData::Input(_) => panic!("not implemented"),
        };
        let mut buffer = String::new();
        let read = take_input.read_to_string(&mut buffer)?;
        if (read as u64) < len {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "string read result is shorter than expected"));
        }
        self.cursor = self.cursor + len;
        Ok(buffer)
    }

    pub fn seek(&mut self, pos: u64) -> io::Result<()> {
        if pos >= self.limit {
            return Err(io::Error::new(
                ErrorKind::UnexpectedEof,
                format!("EOF: seeking position {}, input length: {}", pos, self.limit)));
        }
        // TODO overflow check
        match self.input {
            InputData::File(f) => f.seek(SeekFrom::Start(self.offset + pos))?,
            InputData::Input(_) => panic!("not implemented"),
        };
        self.cursor = pos;
        Ok(())
    }

    pub fn ff(&mut self, len: u64) -> io::Result<()> {
        // TODO maybe implement with SeekFrom::Current?
        self.seek(self.cursor + len)
    }

    pub fn section(&mut self, len: u64) -> Input<'f> {
        Input {
            input: self.input,
            offset: self.offset + self.cursor,
            limit: len,
            cursor: 0,
        }
    }
}

impl<'f> Input<'f> {
    fn quicktime_scan_for_box(&mut self, name: &str,
                              uuid: Option<(u64, u64)>) -> io::Result<Input<'f>> {
        // TODO this infinite loop will throw EOF if the box will not be found
        loop {
            let mut box_length: u64 = self.read_u32(&Endianness::Big)? as u64;
            let box_type: String = self.read_string(4)?;
            // checking for large box:
            if box_length == 1 {
                box_length = self.read_u64(&Endianness::Big)?;
                // box length includes header, have to make adjustments:
                // 4 bytes for box length
                // 4 bytes for box type
                // 8 bytes for box large length
                box_length = box_length - 16;
            } else {
                // box length includes header, have to make adjustments:
                // 4 bytes for box length
                // 4 bytes for box type
                box_length = box_length - 8;
            }
            if &box_type == name {
                match uuid {
                    None => return Ok(self.section(box_length)),
                    Some(u) => {
                        let msb = self.read_u64(&Endianness::Big)?;
                        let lsb = self.read_u64(&Endianness::Big)?;
                        if u.0 == msb && u.1 == lsb {
                            box_length = box_length - 16;
                            return Ok(self.section(box_length));
                        }
                    }
                }
            }
            self.ff(box_length)?;
        }
    }

    pub fn quicktime_search_box(&mut self, box_name: &str) -> io::Result<Input> {
        self.quicktime_scan_for_box(box_name, None)
    }

    pub fn quicktime_search_uuid_box(&mut self, box_uuid: (u64, u64)) -> io::Result<Input> {
        self.quicktime_scan_for_box(&"uuid", Some(box_uuid))
    }
}

fn main() -> io::Result<()> {
    let f = File::open("../DJI_0034.MP4")?;
    let mut input = Input::create(&f);
    let mut moov_box = input.quicktime_search_box("moov")?;
    let mut mvhd_box = moov_box.quicktime_search_box("mvhd")?;
    println!("mvhd box found!");
    Ok(())
}
