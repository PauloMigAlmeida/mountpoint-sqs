use std::collections::BTreeMap;
use std::time::UNIX_EPOCH;

use fuser::{FileAttr, FileType};
use libc::{getgid, getuid};

use crate::sqs::SQSClient;

pub struct Metadata {
    pub file_name: String,
    pub file_attr: FileAttr,
}

pub struct SQSFileSystem {
    superblock: BTreeMap<u64, Metadata>,
    aux_map: BTreeMap<String, u64>,
    sqsclient: SQSClient,
}

impl Default for SQSFileSystem {
    fn default() -> Self {
        SQSFileSystem {
            superblock: BTreeMap::new(),
            aux_map: BTreeMap::new(),
            sqsclient: SQSClient::new(),
        }
    }
}

impl SQSFileSystem {
    fn refresh(&mut self) {
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
    }
    pub fn list_files(&mut self) -> Vec<&Metadata> {
        let mut files = vec![];
        // refresh cache if needed
        //TODO implement logic - for now it will update cache every time
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
