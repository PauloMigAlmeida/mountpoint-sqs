use std::ffi::OsStr;
use std::time::{Duration, SystemTime};

use fuser::{Filesystem, FileType, ReplyAttr, ReplyDirectory, ReplyEntry, ReplyOpen, ReplyWrite, Request, TimeOrNow};
use log::{debug, info, warn};

use crate::cli::CliArgs;
use crate::filesystem::{Metadata, SQSFileSystem};

pub struct SQSFuse {
    sqs_fs: SQSFileSystem,
    default_ttl: Duration,
    // next_file_handle: AtomicU64,
}

impl SQSFuse {
    pub fn new(cli_args: CliArgs) -> Self {
        SQSFuse {
            default_ttl: Duration::from_secs(cli_args.cache_ttl_in_secs),
            sqs_fs: SQSFileSystem::new(cli_args),
            // next_file_handle: AtomicU64::default(),
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
            reply.error(libc::ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        info!("getattr: ino:{ino}");
        let file_metadata = self.sqs_fs.find_by_inode(ino);

        if file_metadata.is_some() {
            let metadata = file_metadata.unwrap();
            reply.attr(&self.default_ttl, &metadata.file_attr);
        } else {
            reply.error(libc::ENOENT);
        }
    }

    fn setattr(&mut self, _req: &Request<'_>, ino: u64, mode: Option<u32>, uid: Option<u32>, gid: Option<u32>, size: Option<u64>, atime: Option<TimeOrNow>, mtime: Option<TimeOrNow>, _ctime: Option<SystemTime>, fh: Option<u64>, _crtime: Option<SystemTime>, _chgtime: Option<SystemTime>, _bkuptime: Option<SystemTime>, flags: Option<u32>, reply: ReplyAttr) {
        debug!(
            "setattr(ino: {:#x?}, mode: {:?}, uid: {:?}, \
            gid: {:?}, size: {:?}, fh: {:?}, flags: {:?})",
            ino, mode, uid, gid, size, fh, flags
        );

        let metadata = match self.sqs_fs.find_by_inode(ino) {
            Some(metadata) => metadata,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        if mode.is_some() {
            warn!("chmod() isn't supported - \
                files given the same uid/gid of the user whom mounted sqsfs");
            reply.error(libc::ENOSYS);
            return;
        }

        if size.is_some() {
            warn!("truncate() or O_TRUNC flag aren't supported as this doesn't make much sense \
            the SQS queues context. Ignoring operation....");
        }

        if atime.is_some() || mtime.is_some() {
            warn!("utimens() isn't supported");
            reply.error(libc::ENOSYS);
            return;
        }

        reply.attr(&Duration::new(0, 0), &metadata.file_attr.into());
        return;
    }

    /// Open a file.
    /// Open flags (with the exception of O_CREAT, O_EXCL, O_NOCTTY and O_TRUNC) are
    /// available in flags. Filesystem may store an arbitrary file handle (pointer, index,
    /// etc) in fh, and use this in other all other file operations (read, write, flush,
    /// release, fsync). Filesystem may also implement stateless file I/O and not store
    /// anything in fh. There are also some flags (direct_io, keep_cache) which the
    /// filesystem may set, to change the way the file is opened. See fuse_file_info
    /// structure in <fuse_common.h> for more details.
    fn open(&mut self, _req: &Request<'_>, ino: u64, flags: i32, reply: ReplyOpen) {
        debug!(
            "open(ino: {:#x?}, flags: {:#x?})",
            ino,
            flags,
        );

        // Check access mode
        let access_mask = match flags & libc::O_ACCMODE {
            libc::O_RDONLY => {
                // Behavior is undefined, but most filesystems return EACCES
                if flags & libc::O_TRUNC != 0 {
                    reply.error(libc::EACCES);
                    return;
                }
                libc::R_OK as u16
            }
            libc::O_WRONLY => libc::W_OK as u16,
            libc::O_RDWR => libc::R_OK as u16 | libc::W_OK as u16,
            // Exactly one access mode flag must be specified
            _ => {
                reply.error(libc::EINVAL);
                return;
            }
        };


        // Check if file exists
        let metadata = match self.sqs_fs.find_by_inode(ino) {
            Some(metadata) => metadata,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        // Check if user has sufficient permissions
        if !check_access(metadata, _req, access_mask) {
            reply.error(libc::EACCES);
            return;
        }

        // create file handle
        let fh = self.sqs_fs.create_file_handler(ino, access_mask);
        reply.opened(fh, 0);
    }

    /// Write data.
    /// Write should return exactly the number of bytes requested except on error. An
    /// exception to this is when the file has been opened in 'direct_io' mode, in
    /// which case the return value of the write system call will reflect the return
    /// value of this operation. fh will contain the value set by the open method, or
    /// will be undefined if the open method didn't set any value.
    ///
    /// write_flags: will contain FUSE_WRITE_CACHE, if this write is from the page cache. If set,
    /// the pid, uid, gid, and fh may not match the value that would have been sent if write cachin
    /// is disabled
    /// flags: these are the file flags, such as O_SYNC. Only supported with ABI >= 7.9
    /// lock_owner: only supported with ABI >= 7.9
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

        // Was file opened with writting permissions ?
        if !self.sqs_fs.check_file_handler_mode(fh, libc::W_OK as u16) {
            reply.error(libc::EPERM);
            return;
        }

        // SQS accepts UTF-8 messages, can we convert data into utf-8?
        let msg = match String::from_utf8(data.to_vec()) {
            Ok(msg) => msg,
            Err(_) => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        // Check if file exists
        let metadata = match self.sqs_fs.find_by_inode(ino) {
            Some(metadata) => metadata.clone(),
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        // Send data to SQS
        let written = match self.sqs_fs.write(&metadata, msg.as_str()) {
            Ok(written) => written,
            Err(_) => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        reply.written(written);
    }

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
            reply.error(libc::ENOENT);
            return;
        }

        let mut entries = vec![
            (1, FileType::Directory, ".".to_string()),
            (1, FileType::Directory, "..".to_string()),
        ];

        for file in self.sqs_fs.list_files() {
            entries.push((file.file_attr.ino, file.file_attr.kind, file.queue_name.clone()));
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

fn check_access(
    file_metadata: &Metadata,
    req: &Request,
    access_mask: u16,
) -> bool {
    let mut owner = false;
    let mut group = false;
    let mut others = false;

    // root is allowed to read & write anything
    if req.uid() == 0 {
        return true;
    }
    // Scratchpad
    // perm = 0o644 = 0b110100100
    // owner  rw = 6 = 110
    // group  r  = 4 = 100
    // others r  = 4 = 100

    if file_metadata.file_attr.uid == req.uid() {
        owner = access_mask & (file_metadata.file_attr.perm >> 6) > 0;
    } else if file_metadata.file_attr.gid == req.gid() {
        group = access_mask & (file_metadata.file_attr.perm >> 3) > 0;
    } else {
        others = access_mask & (file_metadata.file_attr.perm) > 0;
    }

    return owner | group | others;
}
