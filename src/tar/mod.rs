pub mod headers;
use core::slice;
use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::borrow::ToOwned;
use self::headers::TAR_BLOCKSIZE;

#[derive(Debug)]
pub struct TarEntry<'a> {
    header: headers::TarHeader,
    data: &'a [u8]
}

pub fn find_headers(data: &[u8]) -> Box<Vec<TarEntry>> {
    if data.len() % (TAR_BLOCKSIZE as usize) != 0 {
        panic!("wrong length for tar file");
    }
    let mut data: &mut [u8] = &mut data.to_owned();

    let mut header_blocks = Vec::new();
    let mut contiguous_empty_blocks = 0;
    while contiguous_empty_blocks < 2 {
        let chunk = data.split_at_mut(TAR_BLOCKSIZE as usize);
        data = chunk.1;
        let chunk = chunk.0;
        let mut chunk_a: [u8; TAR_BLOCKSIZE as usize] = [0; TAR_BLOCKSIZE as usize];
        chunk_a.copy_from_slice(chunk);
        match read_header(&chunk_a) {
            HeaderReadOutcome::Header(header) => {
                let mut readcount = 0;
                let data_start = data.as_ptr();
                while readcount < header.size as u64 {
                    readcount += TAR_BLOCKSIZE;
                    let chunk = data.split_at_mut(TAR_BLOCKSIZE as usize);
                    data = chunk.1;
                }
                let data_slice = unsafe { slice::from_raw_parts(data_start, header.size as usize) };
                header_blocks.push(TarEntry {
                    header,
                    data: data_slice
                });
            }
            HeaderReadOutcome::Empty => contiguous_empty_blocks += 1,
        }
    }


    return Box::new(header_blocks);
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
