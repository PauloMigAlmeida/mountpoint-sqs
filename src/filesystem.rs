use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use fuser::{FileAttr, FileType};
use libc::{getgid, getuid};
use log::debug;

use crate::cli::CliArgs;
use crate::sqs;
use crate::sqs::SQSClient;

#[derive(Clone)]
pub struct Metadata {
    pub queue_name: String,
    pub queue_url: String,
    pub file_attr: FileAttr,
}

pub struct SQSFileSystem {
    superblock: BTreeMap<u64, Metadata>,
    aux_map: BTreeMap<String, u64>,
    sqsclient: SQSClient,
    last_refresh: SystemTime,
    cli_args: CliArgs,
}

impl SQSFileSystem {
    pub fn new(cli_args: CliArgs) -> Self {
        SQSFileSystem {
            superblock: BTreeMap::new(),
            aux_map: BTreeMap::new(),
            sqsclient: SQSClient::new(),
            last_refresh: UNIX_EPOCH,
            cli_args,
        }
    }

    fn refresh(&mut self) {
        // check if we need to refresh the cache or if we can use what we have
        if self.last_refresh.elapsed().unwrap() < Duration::from_secs(self.cli_args.cache_ttl_in_secs) {
            debug!("no need to refresh the cache");
            return;
        } else {
            debug!("refreshing the cache");
        }

        // purge local cache
        self.aux_map.clear();
        self.superblock.clear();

        // populate cache
        self.do_refresh();

        // update cache control
        self.last_refresh = SystemTime::now();
    }

    fn do_refresh(&mut self) {
        // add top level directory
        self.superblock.insert(1, Metadata {
            queue_name: ".".to_string(),
            queue_url: "".to_string(),
            file_attr: build_fileattr(1, FileType::Directory),
        });

        // fetch queues
        if let Ok(queues) = self.sqsclient.list_queues() {
            // add queues
            let mut fake_ino = 2u64;
            for queue in queues {
                let queue_name = sqs::get_queue_name(queue.as_str()).unwrap();

                self.superblock.insert(fake_ino, Metadata {
                    queue_name: queue_name.clone(),
                    queue_url: queue,
                    file_attr: build_fileattr(fake_ino, FileType::RegularFile),
                });

                self.aux_map.insert(queue_name, fake_ino);

                fake_ino += 1;
            }
        }
    }
    pub fn list_files(&mut self) -> Vec<&Metadata> {
        let mut files = vec![];

        // refresh cache if needed
        self.refresh();

        for item in self.superblock.values() {
            if item.file_attr.ino != 1 {
                files.push(item)
            }
        }

        files
    }

    pub fn has_file(&self, file_name: &String) -> bool {
        self.aux_map.contains_key(file_name)
    }
    pub fn find_by_name(&mut self, file_name: &String) -> Option<&Metadata> {
        // refresh cache if needed
        self.refresh();

        if !self.has_file(file_name) {
            return None;
        }

        self.superblock.get(self.aux_map.get(file_name).unwrap())
    }

    pub fn find_by_inode(&mut self, inode: u64) -> Option<&Metadata> {
        // refresh cache if needed
        self.refresh();

        self.superblock.get(&inode)
    }

    pub fn write(&mut self, metadata: &Metadata, data: &str) -> anyhow::Result<u32> {
        self.sqsclient.send_message(metadata.queue_url.as_str(), data)
    }
}

fn build_fileattr(inode: u64, kind: FileType) -> FileAttr {
    let size: u64;
    let perm: u16;
    let nlink: u32;
    let blksize: u32 = 512;

    match kind {
        FileType::Directory => {
            size = 0;
            perm = 0o755;
            nlink = 2;
        }
        _ => {
            size = 1 * 1024 * 1024;
            perm = 0o644;
            nlink = 1;
        }
    }

    FileAttr {
        ino: inode,
        size,
        blocks: size / blksize as u64,
        atime: UNIX_EPOCH, // 1970-01-01 00:00:00
        mtime: UNIX_EPOCH,
        ctime: UNIX_EPOCH,
        crtime: UNIX_EPOCH,
        kind,
        perm,
        nlink,
        uid: unsafe { getuid() },
        gid: unsafe { getgid() },
        rdev: 0,
        flags: 0,
        blksize,
    }
}
