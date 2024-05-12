mountpoint-sqs
==============
[![Build](https://github.com/PauloMigAlmeida/mountpoint-sqs/actions/workflows/rust.yml/badge.svg)](https://github.com/PauloMigAlmeida/mountpoint-sqs/actions/workflows/rust.yml)

Mountpoint-SQS is a simple and lightweight filesystem that exposes Amazon Simple Queue Service (SQS) as if it were a
filesystem, adhering to the Unix philosophy of "everything is a file". This allows you to interact with SQS queues using
familiar file system CLI utilities for reading, writing, and listing files.

## Features

* Exposes SQS queues as files.
* Supports standard file operations like read, write.
* Designed to be simple, intuitive, and easy to use.
* Built with FUSE (Filesystem in Userspace).

## Example

```bash
# Mount the SQS queues
./mountpoint-sqs /mnt/sqs

# List all queues and messages
ls /mnt/sqs

# Write a new message to a queue
echo "Hello World" > /mnt/sqs/my_queue

# Read a message from a queue
cat /mnt/sqs/my_queue
```

## Build

Install dependencies:

```bash
# On Fedora
dnf install fuse3 fuse3-devel

# On Ubuntu
apt install fuse3 libfuse3-dev
```

To build it, just run:

```bash
cargo build --release
```

## Run

Usage

```bash
$ ./mountpoint-sqs -h
Usage: mountpoint-sqs [OPTIONS] <MOUNT_POINT>

Arguments:
  <MOUNT_POINT>  Directory to mount the SQS queues at

Options:
  -h, --help     Print help
  -V, --version  Print version

Mount options:
      --auto-unmount  Automatically unmount on process exit

SQS options:
  -c, --cache-ttl-in-secs <CACHE_TTL_IN_SECS>
          How long to keep SQS queues cache locally [default: 30]
```

To unmount it

```bash
fusermount -u /mnt/sqs 
```
