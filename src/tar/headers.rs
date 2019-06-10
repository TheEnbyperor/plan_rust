use alloc::str;
use alloc::string::String;
use core::convert;

pub const TAR_BLOCKSIZE: u64 = 512;

#[derive(Debug)]
pub struct TarHeader {
    pub name: String,
    pub mode: usize,
    pub uid: usize,
    pub gid: usize,
    pub size: usize,
    pub mtime: usize,
    pub chksum: usize,
    pub typeflag: FileType,
    pub linkname: String,
    pub uname: String,
    pub gname: String,
    pub devmajor: String,
    pub devminor: String,
    pub prefix: String,
    pub atime: Option<usize>,
    pub ctime: Option<usize>,
}

#[derive(Debug)]
pub enum FileType {
    Regular,
    HardLink,
    SoftLink,
    CharacterSpecial,
    BlockSpecial,
    Directory,
    Fifo,
    LongLink,
}

impl convert::From<&[u8; TAR_BLOCKSIZE as usize]> for TarHeader {
    fn from(b: &[u8; TAR_BLOCKSIZE as usize]) -> Self {
        let magic = str::from_utf8(&b[257..263]).unwrap();
        if magic != "ustar " {
            panic!("This does not look like a tar archive");
        }
        let _version = str::from_utf8(&b[263..265]).unwrap();

        let parse_octal = |start: usize, length: usize| {
            usize::from_str_radix(str::from_utf8(&b[start..start+length]).unwrap(), 8).unwrap()
        };

        let parse_string = |start: usize, length: usize| {
            String::from(str::from_utf8(&b[start..start+length]).unwrap().trim_end_matches('\0'))
        };

        let parse_time = |start: usize, length: usize| {
            let s = str::from_utf8(&b[start..start+length]).unwrap();
            if s.trim_end_matches('\0') == "" {
                None
            } else {
                Some(usize::from_str_radix(s, 8).unwrap())
            }
        };

        TarHeader {
            name: parse_string(0, 100),
            mode: parse_octal(100, 7),
            uid: parse_octal(108, 7),
            gid: parse_octal(116, 7),
            size: parse_octal(124, 11),
            mtime: parse_time(136, 11).unwrap(),
            chksum: parse_octal(148, 6),
            typeflag: (b[156] as char).into(),
            linkname: parse_string(157, 100),
            uname: parse_string(265, 32),
            gname: parse_string(297, 32),
            devmajor: parse_string(329, 8),
            devminor: parse_string(337, 8),
            prefix: parse_string(345, 131),
            atime: parse_time(476, 11),
            ctime: parse_time(488, 11),
        }
    }
}

impl<'a> convert::From<char> for FileType {
    fn from(s: char) -> FileType {
        match s {
            '0' => FileType::Regular,
            '\0' => FileType::Regular,
            '1' => FileType::HardLink,
            '2' => FileType::SoftLink,
            '3' => FileType::CharacterSpecial,
            '4' => FileType::BlockSpecial,
            '5' => FileType::Directory,
            '6' => FileType::Fifo,
            //'7' => FileType::Reserved,
            'L' => FileType::LongLink,
            _ => {
                panic!("Unhandled typeflag: {}", s);
            }
        }
    }
}

