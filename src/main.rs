//! Offline configuration tool for ATK/VXE magnetic keyboards.

mod actuation;
mod cli;
mod config;
mod device;
mod i18n;
mod light;
mod tweaks;

fn main() -> anyhow::Result<()> {
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
    cli::run()
}
