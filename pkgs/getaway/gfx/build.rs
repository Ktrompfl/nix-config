use std::{env, path::Path, process::Command};

fn main() {
    let out = env::var("OUT_DIR").unwrap();
    for shader in ["quad.vert", "quad.frag"] {
        let source = Path::new("shaders").join(shader);
        println!("cargo:rerun-if-changed={}", source.display());
        let target = Path::new(&out).join(format!("{shader}.spv"));
        let status = Command::new("glslc").arg(&source).arg("-o").arg(&target).status().expect("glslc is required");
        assert!(status.success(), "glslc failed for {shader}");
    }
}
