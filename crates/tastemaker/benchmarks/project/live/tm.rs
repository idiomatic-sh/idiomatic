#[path = "allocator.rs"]
mod allocator;
#[path = "tm_main.rs"]
mod main_body;

fn main() -> std::process::ExitCode {
    main_body::run()
}
