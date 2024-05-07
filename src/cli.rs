use std::path::PathBuf;

use anyhow::anyhow;
use clap::{ArgAction, crate_name, crate_version, Parser};
use fuser::MountOption;
use log::debug;
use procfs::process::Process;

use crate::fuse::SQSFuse;

const MOUNT_OPTIONS_HEADER: &str = "Mount options";
const SQS_OPTIONS_HEADER: &str = "SQS options";

#[derive(Parser, Debug, Clone)]
#[command(version = crate_version!(), bin_name = crate_name!())]
pub struct CliArgs {
    #[arg(
    help = "Directory to mount the SQS queues at",
    value_name = "MOUNT_POINT"
    )]
    mount_point: PathBuf,

    #[arg(
    long,
    help = "Automatically unmount on process exit",
    action = ArgAction::SetTrue,
    help_heading = MOUNT_OPTIONS_HEADER,
    )]
    auto_unmount: bool,

    #[arg(
    long,
    help = "Allow root user to access filesystem",
    action = ArgAction::SetTrue,
    help_heading = MOUNT_OPTIONS_HEADER,
    )]
    allow_root: bool,

    #[arg(
    short,
    long,
    help = "How long to keep SQS queues cache locally",
    default_value = "30",
    help_heading = SQS_OPTIONS_HEADER,
    )]
    pub cache_ttl_in_secs: u64,
}

impl CliArgs {
    fn build_options(&self) -> Vec<MountOption> {
        let mut options = vec![
            MountOption::RW,
            MountOption::FSName("sqsfs".to_string()),
        ];
        if self.auto_unmount {
            options.push(MountOption::AutoUnmount);
        }
        if self.allow_root {
            options.push(MountOption::AllowRoot);
        }
        options
    }
}


// Credits: https://github.com/awslabs/mountpoint-s3/blob/9d22f1f77f232baba714e5775bdef171d77e71d9/mountpoint-s3/src/cli.rs#L939-L970
fn validate_mountpoint(path: &PathBuf) -> anyhow::Result<()> {
    let mount_point = path;

    // This is a best-effort validation, so don't fail if we can't read /proc/self/mountinfo for
    // some reason.
    let mounts = match Process::myself().and_then(|me| me.mountinfo()) {
        Ok(mounts) => mounts,
        Err(e) => {
            debug!("failed to read mountinfo, not checking for existing mounts: {e:?}");
            return Ok(());
        }
    };

    if mounts.0.iter().any(|mount| &mount.mount_point == mount_point) {
        return Err(anyhow!("mount point {} is already mounted", mount_point.display()));
    }

    // we check if path exists after reading procfs on purpose. Turns out that the lack of
    // fuse client transport can make stat() fails which looks like the folder doesn't exist
    // when that is the case, mount point would still be mounted.
    if !mount_point.exists() {
        return Err(anyhow!("mount point {} does not exist", mount_point.display()));
    }

    if !mount_point.is_dir() {
        return Err(anyhow!("mount point {} is not a directory", mount_point.display()));
    }

    Ok(())
}

pub fn main() {
    // parsing arguments
    let args = CliArgs::parse();
    let options = args.build_options();

    // check if mount point isn't mounted already and if target mount point exists
    validate_mountpoint(&args.mount_point).expect("Failure when validating mount point");

    // mount sqsfs
    let fuse_fs = SQSFuse::new(args.clone());
    fuser::mount2(
        fuse_fs,
        args.mount_point,
        &options,
    ).unwrap();
}
