use std::ffi::OsStr;
use std::time::Duration;

use fuser::{Filesystem, FileType, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, ReplyWrite, Request};
use libc::{ENOENT, ENOSYS};
use log::{debug, info};

use crate::cli::CliArgs;
use crate::filesystem::SQSFileSystem;

pub struct SQSFuse {
    sqs_fs: SQSFileSystem,
    default_ttl: Duration,
}

impl SQSFuse {
    pub fn new(cli_args: CliArgs) -> Self {
        SQSFuse {
            default_ttl: Duration::from_secs(cli_args.cache_ttl_in_secs),
            sqs_fs: SQSFileSystem::new(cli_args),
        }
    }
}

impl Filesystem for SQSFuse {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let fname = name.to_os_string().into_string().unwrap();
        if parent == 1 && self.sqs_fs.has_file(&fname) {
            let metadata = self.sqs_fs.find_by_name(&fname).unwrap();
            reply.entry(&self.default_ttl, &metadata.file_attr, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        info!("getattr: ino:{ino}");
        let file_metadata = self.sqs_fs.find_by_inode(ino);

        if file_metadata.is_some() {
            let metadata = file_metadata.unwrap();
            reply.attr(&self.default_ttl, &metadata.file_attr);
        } else {
            reply.error(ENOENT);
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
        info!("readdir ino: {ino} fh: {_fh} offset: {offset}");

        if ino != 1 {
            reply.error(ENOENT);
            return;
        }

        let mut entries = vec![
            (1, FileType::Directory, ".".to_string()),
            (1, FileType::Directory, "..".to_string()),
        ];

        for file in self.sqs_fs.list_files() {
            entries.push((file.file_attr.ino, file.file_attr.kind, file.file_name.clone()));
        }

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            // i + 1 means the index of the next entry
            if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                break;
            }
        }

        reply.ok();
    }

    fn write(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        data: &[u8],
        write_flags: u32,
        flags: i32,
        lock_owner: Option<u64>,
        reply: ReplyWrite,
    ) {
        debug!(
            "write(ino: {:#x?}, fh: {}, offset: {}, data.len(): {}, \
            write_flags: {:#x?}, flags: {:#x?}, lock_owner: {:?})",
            ino,
            fh,
            offset,
            data.len(),
            write_flags,
            flags,
            lock_owner
        );
        reply.error(ENOSYS);
    }
}
