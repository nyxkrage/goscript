extern crate self as goscript_engine;
use crate::ffi::*;
use goscript_vm::value::GosValue;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

#[derive(Ffi)]
pub struct Bits {}

#[ffi_impl]
impl Bits {
    pub fn new(_v: Vec<GosValue>) -> Bits {
        Bits {}
    }

    fn ffi_f32_to_bits(&self, args: Vec<GosValue>) -> GosValue {
        let result = u32::from_be_bytes(args[0].as_float32().to_be_bytes());
        GosValue::Uint32(result)
    }

    fn ffi_f32_from_bits(&self, args: Vec<GosValue>) -> GosValue {
        let result = f32::from_be_bytes(args[0].as_uint32().to_be_bytes());
        GosValue::Float32(result.into())
    }

    fn ffi_f64_to_bits(&self, args: Vec<GosValue>) -> GosValue {
        let result = u64::from_be_bytes(args[0].as_float64().to_be_bytes());
        GosValue::Uint64(result)
    }

    fn ffi_f64_from_bits(&self, args: Vec<GosValue>) -> GosValue {
        let result = f64::from_be_bytes(args[0].as_uint64().to_be_bytes());
        GosValue::Float64(result.into())
    }
}
