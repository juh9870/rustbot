use npm_rs::{NodeEnv, NpmEnv};

fn main() {
    println!("cargo:rerun-if-changed=./archive_viewer/src");
    println!("cargo:rerun-if-changed=./archive_viewer/package.json");
    println!("cargo:rerun-if-changed=./archive_viewer/build-config.mjs");
    let built = NpmEnv::default()
        .with_node_env(&NodeEnv::Development)
        .set_path("./archive_viewer")
        .init_env()
        .install(None)
        .run("build")
        .exec()
        .expect("Failed to build a viewer")
        .success();

    if !built {
        panic!("Failed to build a viewer")
    }
}
