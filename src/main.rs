use std::{error, fs, io, path};
use io::Write;
use fs::File;
use error::Error;
use path::PathBuf;
use anyhow::{Context, Result};

struct Segment {
    base_offset: u32,
    file: File,
    file_path: PathBuf,
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
                    .open(format!("segments/{:020}.log", base_offset))?,
                file_path: PathBuf::from(format!("segments/{:020}.log", base_offset)),
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

    fn push(&mut self, msg: Vec<u8>) -> Result<u32, Box<dyn Error>> {

        // We borrow from the segments only to take the size of the last segment and give the borrow back
        let last_seg_size = self.segments
            .last_mut()
            .ok_or_else(|| std::io::Error::other("no segment found"))?
            .size; 

        // Check how many bytes are left in the segment and compare it to the mesage
        let bytes_to_write = msg.len();
        let bytes_written;

        if last_seg_size + bytes_to_write > self.segment_size && last_seg_size > 0 {
            // println!("Not enough space on the segment to write the message!");
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
        

        // println!("Wrote {} bytes", bytes_written);
        // println!("Bytes left in the current segment: {}", self.segment_size - last_seg.size);
        let assigned_offset = self.next_offset;
        self.next_offset += 1;

        Ok(assigned_offset)
    }

    fn read(&self, offset: u32) -> Result<Option<Vec<u8>>>{

        if self.segments.len() == 0 {
            anyhow::bail!("No segments to read from!");
        }

        // Get the segment file
        let wanted_seg = self.segments
            .iter()
            .rev()
            .find(|seg| seg.base_offset <= offset);


        // Iterate over the bytes until we get to the message
        if let Some(seg) = wanted_seg {
            println!("Found segment with path: {:?}", &seg.file_path);
            let seg_bytes = fs::read(&seg.file_path)?;
            let num_msgs = offset - seg.base_offset;
            let mut counter = 0;
            for _ in  0..num_msgs {
                // Get the length of the message
                let payload: [u8; 4] = seg_bytes[counter..counter + 4].try_into()?;
                let msg_size = u32::from_le_bytes(payload) as usize;

                // Skip the message
                counter += 4 + msg_size;
            }

            let payload: [u8; 4] = seg_bytes[counter..counter + 4].try_into()?;
            let msg_size = u32::from_le_bytes(payload) as usize;
            let msg_bytes = seg_bytes[counter + 4..counter + 4 + msg_size].to_vec();
            Ok(Some(msg_bytes))
        }
        else {
            Ok(None)
        }
    }

}

fn main() {
    let seg_size: usize = 100;
    let mut log = Log::new(seg_size).expect("Failed to create the log");

    let msg = String::from("hellol");
    let mut msg_bytes = msg.as_bytes().to_vec();
    let mut length_bytes = (msg.len() as u32).to_le_bytes().to_vec();
    length_bytes.append(&mut msg_bytes);

    for _i in 0..100 {
        log.push(length_bytes.clone()).expect("Failed to push the message");
    }

    let msg_bytes = log.read(25).expect("Error reading the message");
    if let Some(bytes) = msg_bytes {
        let msg = String::from_utf8(bytes)
        .expect("The message is not a valid UTF-8 String");
        println!("{}", msg);
    }
    else {
        println!("No message found for offset {}", 25)
    }
}