use std::{path::Path, fs};

use npm_rs::{NodeEnv, NpmEnv};

fn main() {
    println!("cargo:rerun-if-changed=./archive_viewer/src");
    println!("cargo:rerun-if-changed=./archive_viewer/package.json");
    println!("cargo:rerun-if-changed=./archive_viewer/build-config.mjs");
    let ci_file: &Path = "./archive_viewer/ci/archive.html".as_ref();
    if ci_file.exists() {
        fs::create_dir_all("./archive_viewer/dist").expect("Failed to create dist directory for CI file");
        // ci_file
        fs::rename(ci_file, "./archive_viewer/dist/archive.html").expect("Failed to move CI file to dist location");
        return
    }
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
