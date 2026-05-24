fn main() -> std::process::ExitCode {
    env_logger::init();
    etac_driver::run(etac_session::parse_flags())
}
