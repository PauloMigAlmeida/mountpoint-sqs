mod cli;
mod fuse;
mod sqs;
mod filesystem;

fn main() {
    // Init logging
    env_logger::init();
    // Launch fuse client
    cli::main()
}
