pub mod headers;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::convert::TryInto;
use alloc::borrow::ToOwned;
use self::headers::TAR_BLOCKSIZE;
use crate::println;

pub fn find_headers(data: &[u8]) -> <Box<Vec<usize>> {
    if data.len() % (TAR_BLOCKSIZE as usize) != 0 {
        panic!("wrong length for tar file");
    }
    let mut data: &mut [u8] = &mut data.to_owned();

    let mut header_blocks = Vec::new();
    let mut contiguous_empty_blocks = 0;
    let mut block_index = 0;
    while contiguous_empty_blocks < 2 {
        let chunk = data.split_at_mut(TAR_BLOCKSIZE as usize);
        data = chunk.1;
        let chunk = chunk.0;
        let mut chunk_a: [u8; TAR_BLOCKSIZE as usize] = [0; TAR_BLOCKSIZE as usize];
        chunk_a.copy_from_slice(chunk);
        match read_header(&chunk_a) {
            HeaderReadOutcome::Header(header) => {
                header_blocks.push(block_index);
                let mut readcount = 0;
                block_index += 1;
                while readcount < header.size as u64 {
                    block_index += 1;
                    readcount += TAR_BLOCKSIZE;
                    let chunk = data.split_at_mut(TAR_BLOCKSIZE as usize);
                    data = chunk.1;
                    let chunk = chunk.0;
                }
            }
            HeaderReadOutcome::Empty => contiguous_empty_blocks += 1,
        }
    }


    return Ok(Box::new(header_blocks));
}

enum HeaderReadOutcome {
    Header(headers::TarHeader),
    Empty,
}

fn read_header(chunk: &[u8; TAR_BLOCKSIZE as usize]) -> HeaderReadOutcome {
    if chunk.iter().all(|i| *i == 0) {
        return HeaderReadOutcome::Empty;
    }

    HeaderReadOutcome::Header(chunk.into())
}
