use std::{fs, io, path};
use io::Write;
use fs::File;
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
        let file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(format!("segments/{:020}.log", base_offset))?;
        let size = file.metadata()?.len() as usize;
        Ok (
            Segment {
                base_offset,
                file,
                file_path: PathBuf::from(format!("segments/{:020}.log", base_offset)),
                size
            }
        )
    }
}

struct Log {
    segments: Vec<Segment>,
    next_offset: u32,
    segment_size: usize,
    broken: bool,
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
            broken: false,
        };

        Ok(log)
    }

    fn push(&mut self, msg: Vec<u8>) -> Result<u32> {

        // We can write to the log only if it's not broken
        if self.broken {
            anyhow::bail!("The log is broken for now...")
        }

        // We borrow from the segments only to take the size of the last segment and give the borrow back
        let last_seg_size = self.segments
            .last_mut()
            .ok_or_else(|| std::io::Error::other("no segment found"))?
            .size; 

        // Check how many bytes are left in the segment and compare it to the mesage
        let bytes_to_write = msg.len();

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

        if let Err(write_err) = last_seg.file.write_all(&msg) {
            if let Err(truncate_err) = last_seg.file.set_len(last_seg.size as u64) {
                self.broken = true;
                anyhow::bail!("Both writing and truncating the file failed! \n Traceback: {:?} \n {:?}",
                    write_err, 
                    truncate_err
                )
            }
            anyhow::bail!("Writing the message failed, rolled the file back... \n Traceback: {:?}",
                write_err
            );
        } else {
            last_seg.file.sync_data()?;

            last_seg.size += bytes_to_write;
            // println!("Wrote {} bytes", bytes_to_write);
            // println!("Bytes left in the current segment: {}", self.segment_size - last_seg.size);
            let assigned_offset = self.next_offset;
            self.next_offset += 1;

            Ok(assigned_offset)
        }
        
    }

    fn read(&self, offset: u32) -> Result<Option<Vec<u8>>>{

        fn next_msg(buffer: &[u8], header_pos: usize) -> Result<(usize, &[u8])> {
            let length_bytes: [u8; 4] = buffer[header_pos..header_pos + 4].try_into()?;
            let msg_start_byte = header_pos + 4;
            let length = u32::from_le_bytes(length_bytes) as usize;

            Ok((msg_start_byte + length, &buffer[msg_start_byte..msg_start_byte + length]))
        }

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

            let mut pos = 0;
            for _ in  0..num_msgs {
                // Skip all the messages, ignore the slices
                (pos, _) = next_msg(&seg_bytes, pos)?;
            }
            // Get the bytes of the message we need
            let (_, msg_bytes) = next_msg(&seg_bytes, pos)?;
            Ok(Some(msg_bytes.to_vec()))
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

    let msg_bytes = log.read(30).expect("Error reading the message");
    if let Some(bytes) = msg_bytes {
        let msg = String::from_utf8(bytes)
        .expect("The message is not a valid UTF-8 String");
        println!("{}", msg);
    }
    else {
        println!("No message found for offset {}", 25)
    }
}