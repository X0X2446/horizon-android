//! Stub for host-build verification. The real crate comes from crates.io.
#![allow(dead_code)]

pub mod objects {
    pub struct JClass;
    pub struct JString;
}

pub struct JNIEnv;
pub mod sys {
    pub type jint = i32;
    pub type jfloat = f32;
    pub type jlong = i64;
}
