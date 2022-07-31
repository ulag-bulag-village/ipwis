fn main() -> ::ipwis_modules_task_builder::BuildResult {
    let src = "./target/wasm32-wasi/release/ipwis_modules_task_demo.wasm";
    let dst = "output.wasm";

    ::ipwis_modules_task_builder::try_build_wasi(src, dst)
}
