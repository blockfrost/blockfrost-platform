fn main() {
    // TODO: https://github.com/txpipe/dolos/issues/508
    if !cfg!(windows) {
        build_utils::dolos::fetch_binary();
    }
}
