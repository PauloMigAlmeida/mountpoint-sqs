use std::ffi::OsStr;
use std::time::{Duration, UNIX_EPOCH};

use fuser::{
    FileAttr, Filesystem, FileType, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, Request,
};
use libc::ENOENT;
use log::info;

use crate::cli::CliArgs;
use crate::filesystem::SQSFileSystem;

const TTL: Duration = Duration::from_secs(1); // 1 second

const HELLO_DIR_ATTR: FileAttr = FileAttr {
    ino: 1,
    size: 0,
    blocks: 0,
    atime: UNIX_EPOCH, // 1970-01-01 00:00:00
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o755,
    nlink: 2,
    uid: 1000, //TODO fix this
    gid: 1000,
    rdev: 0,
    flags: 0,
    blksize: 512,
};

// const HELLO_TXT_CONTENT: &str = "Hello World!\n";


pub struct SQSFuse {
    sqs_fs: SQSFileSystem,
}

impl SQSFuse {
    pub fn new(cli_args: CliArgs) -> Self {
        SQSFuse {
            sqs_fs: SQSFileSystem::new(cli_args),
        }
    }
}

impl Filesystem for SQSFuse {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let fname = name.to_os_string().into_string().unwrap();
        if parent == 1 && self.sqs_fs.has_file(&fname) {
            let metadata = self.sqs_fs.find_by_name(&fname).unwrap();
            reply.entry(&TTL, &metadata.file_attr, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        match ino {
            1 => reply.attr(&TTL, &HELLO_DIR_ATTR),
            _ => reply.error(ENOENT),
        }
    }
    //
    // fn read(
    //     &mut self,
    //     _req: &Request,
    //     ino: u64,
    //     _fh: u64,
    //     offset: i64,
    //     _size: u32,
    //     _flags: i32,
    //     _lock: Option<u64>,
    //     reply: ReplyData,
    // ) {
    //     if ino == 2 {
    //         reply.data(&HELLO_TXT_CONTENT.as_bytes()[offset as usize..]);
    //     } else {
    //         reply.error(ENOENT);
    //     }
    // }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        info!("readdir being called");

        if ino != 1 {
            reply.error(ENOENT);
            return;
        }

        let mut entries = vec![
            (1, FileType::Directory, ".".to_string()),
            (1, FileType::Directory, "..".to_string()),
        ];

        for file in self.sqs_fs.list_files() {
            entries.push((file.file_attr.ino, FileType::RegularFile, file.file_name.clone()));
        }

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            // i + 1 means the index of the next entry
            if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                break;
            }
        }

        reply.ok();
    }
}
