use crate::filesystem::SimpleQueueServiceFileSystem;

mod cli;
mod filesystem;

fn main() {
    // Init logging
    env_logger::init();
    // Launch fuse client
    cli::main(SimpleQueueServiceFileSystem)
}
