extern crate self as goscript_engine;
use crate::ffi::*;
use goscript_vm::instruction::ValueType;
use goscript_vm::metadata::GosMetadata;
use goscript_vm::objects::MetadataObjs;
use goscript_vm::value::{GosValue, IfaceUnderlying, PointerObj, UserData};
use std::any::Any;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

const WRONG_TYPE_MSG: &str = "reflect: wrong type";

macro_rules! params_as_std_val {
    ($params:expr) => {{
        let ud = $params[0].as_pointer().as_user_data();
        ud.as_any().downcast_ref::<StdValue>().unwrap()
    }};
}

macro_rules! wrap_std_val {
    ($v:expr, $metas:expr) => {
        GosValue::new_pointer(PointerObj::UserData(Rc::new(StdValue::new($v, &$metas))))
    };
}

macro_rules! meta_objs {
    ($ptr:expr) => {
        unsafe { &*$ptr }
    };
}

macro_rules! err_wrong_type {
    () => {
        Err(WRONG_TYPE_MSG.to_string())
    };
}

enum GosKind {
    Invalid = 0,
    Bool,
    Int,
    Int8,
    Int16,
    Int32,
    Int64,
    Uint,
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    _Uintptr, // do not support for now
    Float32,
    Float64,
    Complex64,
    Complex128,
    Array,
    Chan,
    Func,
    Interface,
    Map,
    Ptr,
    Slice,
    String,
    Struct,
    UnsafePointer,
}

#[derive(Ffi)]
pub struct Reflect {}

#[ffi_impl]
impl Reflect {
    pub fn new(_v: Vec<GosValue>) -> Reflect {
        Reflect {}
    }

    fn ffi_value_of(&self, ctx: &FfiCallCtx, params: Vec<GosValue>) -> GosValue {
        StdValue::value_of(&params[0], ctx)
    }

    fn ffi_type_of(&self, ctx: &FfiCallCtx, params: Vec<GosValue>) -> Vec<GosValue> {
        let v = params_as_std_val!(params);
        let (t, k) = StdType::type_of(&v.val, ctx);
        vec![t, k]
    }

    fn ffi_bool_val(&self, params: Vec<GosValue>) -> RuntimeResult<GosValue> {
        params_as_std_val!(params).bool_val()
    }

    fn ffi_int_val(&self, params: Vec<GosValue>) -> RuntimeResult<GosValue> {
        params_as_std_val!(params).int_val()
    }

    fn ffi_uint_val(&self, params: Vec<GosValue>) -> RuntimeResult<GosValue> {
        params_as_std_val!(params).uint_val()
    }

    fn ffi_float_val(&self, params: Vec<GosValue>) -> RuntimeResult<GosValue> {
        params_as_std_val!(params).float_val()
    }

    fn ffi_bytes_val(&self, params: Vec<GosValue>) -> RuntimeResult<GosValue> {
        params_as_std_val!(params).bytes_val()
    }

    fn ffi_elem(&self, ctx: &FfiCallCtx, params: Vec<GosValue>) -> RuntimeResult<GosValue> {
        params_as_std_val!(params).elem(ctx)
    }

    fn ffi_field(&self, ctx: &FfiCallCtx, params: Vec<GosValue>) -> RuntimeResult<GosValue> {
        params_as_std_val!(params).field(ctx, &params[1])
    }
}

#[derive(Clone, Debug)]
struct StdValue {
    val: GosValue,
    mobjs: *const MetadataObjs,
}

impl UserData for StdValue {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl StdValue {
    fn new(v: GosValue, objs: &MetadataObjs) -> StdValue {
        StdValue {
            val: v,
            mobjs: objs,
        }
    }

    fn value_of(v: &GosValue, ctx: &FfiCallCtx) -> GosValue {
        let iface = v.as_interface().borrow();
        let v = match &iface.underlying() {
            IfaceUnderlying::Gos(v, _) => v.clone(),
            // todo: should we return something else?
            IfaceUnderlying::Ffi(_) => GosValue::Nil(iface.meta),
            IfaceUnderlying::None => GosValue::Nil(iface.meta),
        };
        wrap_std_val!(v, &ctx.vm_objs.metas)
    }

    fn bool_val(&self) -> RuntimeResult<GosValue> {
        match &self.val {
            GosValue::Bool(_) => Ok(self.val.clone()),
            _ => err_wrong_type!(),
        }
    }

    fn int_val(&self) -> RuntimeResult<GosValue> {
        match &self.val {
            GosValue::Int(i) => Ok(*i as i64),
            GosValue::Int8(i) => Ok(*i as i64),
            GosValue::Int16(i) => Ok(*i as i64),
            GosValue::Int32(i) => Ok(*i as i64),
            GosValue::Int64(i) => Ok(*i),
            _ => err_wrong_type!(),
        }
        .map(|x| GosValue::Int64(x))
    }

    fn uint_val(&self) -> RuntimeResult<GosValue> {
        match &self.val {
            GosValue::Uint(i) => Ok(*i as u64),
            GosValue::Uint8(i) => Ok(*i as u64),
            GosValue::Uint16(i) => Ok(*i as u64),
            GosValue::Uint32(i) => Ok(*i as u64),
            GosValue::Uint64(i) => Ok(*i),
            _ => err_wrong_type!(),
        }
        .map(|x| GosValue::Uint64(x))
    }

    fn float_val(&self) -> RuntimeResult<GosValue> {
        match &self.val {
            GosValue::Float32(f) => Ok((Into::<f32>::into(*f) as f64).into()),
            GosValue::Float64(f) => Ok(*f),
            _ => err_wrong_type!(),
        }
        .map(|x| GosValue::Float64(x))
    }

    fn bytes_val(&self) -> RuntimeResult<GosValue> {
        match &self.val {
            GosValue::Slice(s) => {
                let metas = meta_objs!(self.mobjs);
                let (m, _) = metas[s.0.meta.as_non_ptr()].as_slice_or_array();
                match m.get_value_type(metas) {
                    ValueType::Uint8 => Ok(self.val.clone()),
                    _ => err_wrong_type!(),
                }
            }
            _ => err_wrong_type!(),
        }
    }

    fn elem(&self, ctx: &FfiCallCtx) -> RuntimeResult<GosValue> {
        match &self.val {
            GosValue::Interface(iface) => Ok(iface
                .borrow()
                .underlying_value()
                .map(|x| x.clone())
                .unwrap_or(GosValue::new_nil())),
            GosValue::Pointer(p) => Ok(p.deref(&ctx.stack, &ctx.vm_objs.packages)),
            _ => err_wrong_type!(),
        }
        .map(|x| wrap_std_val!(x, &ctx.vm_objs.metas))
    }

    fn field(&self, ctx: &FfiCallCtx, ival: &GosValue) -> RuntimeResult<GosValue> {
        let i = *ival.as_int() as usize;
        match self.val.try_as_struct() {
            Some(s) => {
                let fields = &s.0.borrow().fields;
                if fields.len() <= i {
                    Err("reflect: Field index out of range".to_string())
                } else {
                    Ok(fields[i].clone())
                }
            }
            None => err_wrong_type!(),
        }
        .map(|x| wrap_std_val!(x, &ctx.vm_objs.metas))
    }
}

#[derive(Clone, Debug)]
struct StdType {
    meta: GosMetadata,
    mobjs: *const MetadataObjs,
}

impl UserData for StdType {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq(&self, other: &dyn UserData) -> bool {
        match other.as_any().downcast_ref::<StdType>() {
            Some(other_type) => {
                let objs = meta_objs!(self.mobjs);
                self.meta.identical(&other_type.meta, objs)
            }
            None => false,
        }
    }
}

impl StdType {
    fn new(m: GosMetadata, objs: &MetadataObjs) -> StdType {
        StdType {
            meta: m,
            mobjs: objs,
        }
    }

    fn type_of(val: &GosValue, ctx: &FfiCallCtx) -> (GosValue, GosValue) {
        let m = val.get_meta(ctx.vm_objs, ctx.stack);
        let typ = StdType::new(m, &ctx.vm_objs.metas);
        let kind = match m
            .get_underlying(&ctx.vm_objs.metas)
            .get_value_type(&ctx.vm_objs.metas)
        {
            ValueType::Bool => GosKind::Bool,
            ValueType::Int => GosKind::Int,
            ValueType::Int8 => GosKind::Int8,
            ValueType::Int16 => GosKind::Int16,
            ValueType::Int32 => GosKind::Int32,
            ValueType::Int64 => GosKind::Int64,
            ValueType::Uint => GosKind::Uint,
            ValueType::Uint8 => GosKind::Uint8,
            ValueType::Uint16 => GosKind::Uint16,
            ValueType::Uint32 => GosKind::Uint32,
            ValueType::Uint64 => GosKind::Uint64,
            ValueType::Float32 => GosKind::Float32,
            ValueType::Float64 => GosKind::Float64,
            ValueType::Complex64 => GosKind::Complex64,
            ValueType::Complex128 => GosKind::Complex128,
            ValueType::Array => GosKind::Array,
            ValueType::Channel => GosKind::Chan,
            ValueType::Closure => GosKind::Func,
            ValueType::Interface => GosKind::Interface,
            ValueType::Map => GosKind::Map,
            ValueType::Pointer => {
                let ptr: &PointerObj = &*val.as_pointer();
                match ptr {
                    PointerObj::UserData(_) => GosKind::UnsafePointer,
                    _ => GosKind::Ptr,
                }
            }
            ValueType::Slice => GosKind::Slice,
            ValueType::Str => GosKind::String,
            ValueType::Struct => GosKind::Struct,
            _ => GosKind::Invalid,
        };
        (
            GosValue::new_pointer(PointerObj::UserData(Rc::new(typ))),
            GosValue::Uint(kind as usize),
        )
    }
}
