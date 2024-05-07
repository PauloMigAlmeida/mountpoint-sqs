use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use fuser::{FileAttr, FileType};
use libc::{getgid, getuid};
use log::info;

use crate::cli::CliArgs;
use crate::sqs::SQSClient;

pub struct Metadata {
    pub file_name: String,
    pub file_attr: FileAttr,
}

pub struct SQSFileSystem {
    superblock: BTreeMap<u64, Metadata>,
    aux_map: BTreeMap<String, u64>,
    sqsclient: SQSClient,
    last_cache_refresh: SystemTime,
    cli_args: CliArgs,
}

impl SQSFileSystem {
    pub fn new(cli_args: CliArgs) -> Self {
        SQSFileSystem {
            superblock: BTreeMap::new(),
            aux_map: BTreeMap::new(),
            sqsclient: SQSClient::new(),
            last_cache_refresh: UNIX_EPOCH,
            cli_args,
        }
    }

    fn refresh(&mut self) {
        // check if we need to refresh the cache or if we can use what we have
        if self.last_cache_refresh.elapsed().unwrap() < Duration::from_secs(self.cli_args.cache_ttl_in_secs) {
            info!("no need to refresh the cache");
            return;
        } else {
            info!("refreshing the cache");
        }

        // purge local cache
        self.aux_map.clear();
        self.superblock.clear();

        // fetch fresh ones
        let queues = self.sqsclient.list_queues();

        // populate cache
        let mut fake_ino = 2u64;
        for queue in queues.unwrap() {
            self.superblock.insert(fake_ino, Metadata {
                file_name: queue.clone(),
                file_attr: FileAttr {
                    ino: fake_ino,
                    size: 13,
                    blocks: 1,
                    atime: UNIX_EPOCH, // 1970-01-01 00:00:00
                    mtime: UNIX_EPOCH,
                    ctime: UNIX_EPOCH,
                    crtime: UNIX_EPOCH,
                    kind: FileType::RegularFile,
                    perm: 0o644,
                    nlink: 1,
                    uid: unsafe { getuid() },
                    gid: unsafe { getgid() },
                    rdev: 0,
                    flags: 0,
                    blksize: 512,
                },
            });

            self.aux_map.insert(queue, fake_ino);

            fake_ino += 1;
        }

        // update cache control
        self.last_cache_refresh = SystemTime::now();
    }
    pub fn list_files(&mut self) -> Vec<&Metadata> {
        let mut files = vec![];

        // refresh cache if needed
        self.refresh();

        for item in self.superblock.values() {
            files.push(item)
        }

        files
    }

    pub fn has_file(&self, file_name: &String) -> bool {
        self.aux_map.contains_key(file_name)
    }
    pub fn find_by_name(&self, file_name: &String) -> Option<&Metadata> {
        if !self.has_file(file_name) {
            return None;
        }

        self.superblock.get(self.aux_map.get(file_name).unwrap())
    }
}
