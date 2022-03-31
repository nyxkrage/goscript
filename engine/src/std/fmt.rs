extern crate self as goscript_engine;
use crate::ffi::*;
use goscript_vm::value::GosValue;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

#[derive(Ffi)]
pub struct Fmt {}

#[ffi_impl]
impl Fmt {
    pub fn new(_v: Vec<GosValue>) -> Fmt {
        Fmt {}
    }

    fn ffi_println(&self, args: Vec<GosValue>) {
        let vec = args[0].as_slice().0.get_vec();
        let strs: Vec<String> = vec
            .iter()
            .map(|x| {
                if x.is_nil() {
                    "<nil>".to_owned()
                } else {
                    match x.iface_underlying() {
                        Some(v) => v.to_string(),
                        None => "<ffi>".to_owned(),
                    }
                }
            })
            .collect();
        println!("{}", strs.join(", "));
    }

    fn ffi_printf(&self, args: Vec<GosValue>) {
        let mut vec = args[0].as_slice().0.get_vec();
        let fmt_str = vec.remove(0).iface_underlying().expect("bro?").to_string();
        let fmt_str = fmt_str.as_ref();
        let mut box_args: Vec<Box<dyn sprintf::Printf>> = Vec::new();
        for x in vec {
            if x.is_nil() {
                box_args.push(Box::new(NilType()));
            } else {
                match x.iface_underlying() {
                    Some(i) => match i {
                        GosValue::Nil(_) => {
                            box_args.push(Box::new(NilType()));
                        },
                        GosValue::Bool(v) => {
                            box_args.push(Box::new(BoolType(v)));
                        },
                        GosValue::Int(v) => {
							box_args.push(Box::new(v));
						},
                        GosValue::Int8(v) => {
							box_args.push(Box::new(v));
						},
                        GosValue::Int16(v) => {
							box_args.push(Box::new(v));
						},
                        GosValue::Int32(v) => {
							box_args.push(Box::new(v));
						},
                        GosValue::Int64(v) => {
							box_args.push(Box::new(v));
						},
                        GosValue::Uint(v) => {
							box_args.push(Box::new(v));
						},
                        GosValue::UintPtr(v) => {
							box_args.push(Box::new(v));
						},
                        GosValue::Uint8(v) => {
							box_args.push(Box::new(v));
						},
                        GosValue::Uint16(v) => {
							box_args.push(Box::new(v));
						},
                        GosValue::Uint32(v) => {
							box_args.push(Box::new(v));
						},
                        GosValue::Uint64(v) => {
							box_args.push(Box::new(v));
						},
                        GosValue::Float32(v) => {
							box_args.push(Box::new(v.into_inner()));
						},
                        GosValue::Float64(v) => {
							box_args.push(Box::new(v.into_inner()));
						},
                        GosValue::Complex64(_, _) => {
                            unimplemented!();
						},
                        GosValue::Complex128(_) => {
                            unimplemented!();
						},
                        GosValue::Function(_) => {
                            unimplemented!();
						},
                        GosValue::Package(_) => {
                            unimplemented!();
						},
                        GosValue::Metadata(_) => {
                            unimplemented!();
						},
                        GosValue::Str(_) => {
                            unimplemented!();
						},
                        GosValue::Array(_) => {
                            unimplemented!();
						},
                        GosValue::Pointer(_) => {
                            unimplemented!();
						},
                        GosValue::Closure(_) => {
                            unimplemented!();
						},
                        GosValue::Slice(_) => {
                            unimplemented!();
						},
                        GosValue::Map(_) => {
                            unimplemented!();
						},
                        GosValue::Interface(_) => {
                            unimplemented!();
						},
                        GosValue::Struct(_) => {
                            unimplemented!();
						},
                        GosValue::Channel(_) => {
                            unimplemented!();
						},
                        GosValue::Named(_) => {
                            unimplemented!();
						},
                    },
                    None => {
                        box_args.push(Box::new(NilType()));
                    }
                }
            };
        }
        let fmt_args = box_args.iter().map(Box::as_ref).collect::<Vec<&dyn sprintf::Printf>>();

        let out = sprintf::vsprintf(
            fmt_str,
            &fmt_args
        )
        .unwrap();
        println!("{}", out);
    }
}

#[derive(Clone, Copy)]
struct BoolType(bool);

impl sprintf::Printf for BoolType {
    fn format(&self, _: &sprintf::ConversionSpecifier) -> sprintf::Result<String> {
        Ok(self.0.to_string())
    }

    fn as_int(&self) -> Option<i32> {
        None
    }
}
struct NilType();

impl sprintf::Printf for NilType {
    fn format(&self, _: &sprintf::ConversionSpecifier) -> sprintf::Result<String> {
        Ok("<nil>".to_string())
    }

    fn as_int(&self) -> Option<i32> {
        None
    }
}
