fn main() {
    let os = bf_build_utils::target::os();
    let arch = bf_build_utils::target::arch();

    bf_build_utils::testgen_hs::ensure(os, arch);
}
