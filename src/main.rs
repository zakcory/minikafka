use std::{error, fs, io::Write};
use fs::File;
use error::Error;
use anyhow::{Context, Result};

struct Segment {
    base_offset: u32,
    file: File,
    size: usize    
}

impl Segment {
    fn new(base_offset: u32) -> Result<Self> {
        Ok (
            Segment {
                base_offset,
                file: fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(format!("segments/{}.log", base_offset))?,
                size: 0
            }
        )
    }
}

struct Log {
    segments: Vec<Segment>,
    next_offset: u32,
    segment_size: usize,
}

impl Log {

    fn new(segment_size: usize) -> Result<Self> {

        fs::create_dir_all("segments")
            .context("Unable to create the segments directory")?;

        let log = Log {
            segments: Vec::from([
                Segment::new(0)
                    .context("Failed to create the first segment")?
            ]),
            next_offset: 0,
            segment_size,
        };

        Ok(log)
    }

    fn push(&mut self, msg: Vec<u8>) -> Result<(), Box<dyn Error>> {

        // We borrow from the segments only to take the size of the last segment and give the borrow back
        let last_seg_size = self.segments
                                        .last_mut()
                                        .ok_or_else(|| std::io::Error::other("no segment found"))?
                                        .size; 

        // Check how many bytes are left in the segment and compare it to the mesage
        let bytes_to_write = msg.len();
        let bytes_written;
        println!("Bytes that will be busy after the write: {}", last_seg_size + bytes_to_write);
        if last_seg_size + bytes_to_write > self.segment_size {
            println!("Not enough space on the segment to write the message!");
            let new_seg = Segment::new(self.next_offset)
                    .context(format!("Failed to create the {}.log segment", self.next_offset))?;
            self.segments.push(
                new_seg
            );
        }
        let last_seg = self.segments
                                    .last_mut()
                                    .ok_or_else(|| std::io::Error::other("no segment found"))?;
        bytes_written = last_seg.file.write(&msg)?;
        last_seg.size += bytes_written;
        

        println!("Wrote {} bytes", bytes_written);
        self.next_offset += 1;

        Ok(())
    }

}

fn main() {
    let seg_size: usize = 100;
    let mut log = Log::new(seg_size).expect("Failed to create the log");

    let msg = String::from("hello0");
    let mut msg_bytes = msg.as_bytes().to_vec();
    let mut length_bytes = (msg.len() as u32).to_le_bytes().to_vec();
    msg_bytes.append(&mut length_bytes);

    for i in 0..100 {
        log.push(msg_bytes.clone()).expect("Failed to push the message");
    }
}