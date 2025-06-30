pub fn evaluate(target_os: &str, target_arch: &str) {
    println!("cargo::rustc-check-cfg=cfg(evaluate)");

    if target_os == "windows" {
        println!(
            "cargo:warning=Skipping 'evaluate' cfg for {}-{}",
            target_os, target_arch
        );
    } else {
        println!(
            "cargo:warning=Going to build with 'evaluate' cfg for {}-{}",
            target_os, target_arch
        );
        println!("cargo:rustc-cfg=evaluate");
    }
}