fn main() {
    bf_build_utils::git::set_git_env();
    bf_build_utils::testgen_hs::ensure();
    bf_build_utils::hydra_node::ensure();
}
