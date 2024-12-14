/**
 * 路径相关功能
 */
use std::env;
use std::path;

pub fn __dirname() -> path::PathBuf {
    env!("CARGO_MANIFEST_DIR").into()
}
