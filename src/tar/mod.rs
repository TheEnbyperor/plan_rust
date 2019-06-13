pub mod headers;
use core::slice;
use alloc::vec::Vec;
use alloc::borrow::ToOwned;
use self::headers::TAR_BLOCKSIZE;

#[derive(Debug, Clone)]
pub struct TarEntry<'a> {
    header: headers::TarHeader,
    data: &'a [u8]
}

impl<'a> TarEntry<'a> {
    pub fn header(&self) -> &headers::TarHeader {
        &self.header
    }
    pub fn data(&self) -> &'a [u8] {
        &self.data
    }
}

pub fn find_headers(data: &[u8]) -> Vec<TarEntry> {
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
            Some(header) => {
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
            None => contiguous_empty_blocks += 1,
        }
    }

    header_blocks
}

fn read_header(chunk: &[u8; TAR_BLOCKSIZE as usize]) -> Option<headers::TarHeader> {
    if chunk.iter().all(|i| *i == 0) {
        return None;
    }

    Some(chunk.into())
}
