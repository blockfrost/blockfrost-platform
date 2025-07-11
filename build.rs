fn main() {
    build_utils::git::set_git_env();
    build_utils::testgen_hs::ensure();
}
