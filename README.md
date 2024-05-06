mountpoint-sqs
==============

## Build

On Fedora:

```bash
dnf install fuse3 fuse3-devel
```

To build it, just run:

```bash
cargo build --release
```

## Run

To mount it

```bash
mkdir mnt_sqs
mountpoint-sqs mnt_sqs 
```

To unmount (in another session)

```bash
fusermount -u mnt_sqs 
```
