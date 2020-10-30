#![macro_use]
use super::instruction::{Instruction, OpIndex, Opcode, ValueType};
use super::metadata::*;
use super::value::GosValue;
use goscript_parser::objects::EntityKey;
use slotmap::{new_key_type, DenseSlotMap};
use std::cell::{Ref, RefCell, RefMut};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::hash::Hash;
use std::iter::FromIterator;
use std::rc::{Rc, Weak};

const DEFAULT_CAPACITY: usize = 128;

#[macro_export]
macro_rules! null_key {
    () => {
        slotmap::Key::null()
    };
}

new_key_type! { pub struct MetadataKey; }
new_key_type! { pub struct FunctionKey; }
new_key_type! { pub struct PackageKey; }

pub type InterfaceObjs = Vec<Weak<RefCell<InterfaceObj>>>;
pub type ClosureObjs = Vec<Weak<RefCell<ClosureObj>>>;
pub type SliceObjs = Vec<Weak<SliceObj>>;
pub type MapObjs = Vec<Weak<MapObj>>;
pub type StructObjs = Vec<Weak<RefCell<StructObj>>>;
pub type ChannelObjs = Vec<Weak<RefCell<ChannelObj>>>;
pub type BoxedObjs = Vec<Weak<GosValue>>;
pub type MetadataObjs = DenseSlotMap<MetadataKey, MetadataType>;
pub type FunctionObjs = DenseSlotMap<FunctionKey, FunctionVal>;
pub type PackageObjs = DenseSlotMap<PackageKey, PackageVal>;

pub fn key_to_u64<K>(key: K) -> u64
where
    K: slotmap::Key,
{
    let data: slotmap::KeyData = key.into();
    data.as_ffi()
}

pub fn u64_to_key<K>(u: u64) -> K
where
    K: slotmap::Key,
{
    let data = slotmap::KeyData::from_ffi(u);
    data.into()
}

#[derive(Debug)]
pub struct VMObjects {
    pub interfaces: InterfaceObjs,
    pub closures: ClosureObjs,
    pub slices: SliceObjs,
    pub maps: MapObjs,
    pub structs: StructObjs,
    pub channels: ChannelObjs,
    pub boxed: BoxedObjs,
    pub metas: MetadataObjs,
    pub functions: FunctionObjs,
    pub packages: PackageObjs,
    pub metadata: Metadata,
}

impl VMObjects {
    pub fn new() -> VMObjects {
        let mut metas = DenseSlotMap::with_capacity_and_key(DEFAULT_CAPACITY);
        let md = Metadata::new(&mut metas);
        VMObjects {
            interfaces: vec![],
            closures: vec![],
            slices: vec![],
            maps: vec![],
            structs: vec![],
            channels: vec![],
            boxed: vec![],
            metas: metas,
            functions: DenseSlotMap::with_capacity_and_key(DEFAULT_CAPACITY),
            packages: DenseSlotMap::with_capacity_and_key(DEFAULT_CAPACITY),
            metadata: md,
        }
    }
}

// ----------------------------------------------------------------------------
// StringObj

pub type StringIter<'a> = std::str::Chars<'a>;

pub type StringEnumIter<'a> = std::iter::Enumerate<std::str::Chars<'a>>;

#[derive(Debug)]
pub struct StringObj {
    data: Rc<String>,
    begin: usize,
    end: usize,
}

impl StringObj {
    #[inline]
    pub fn with_str(s: String) -> StringObj {
        let len = s.len();
        StringObj {
            data: Rc::new(s),
            begin: 0,
            end: len,
        }
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.data.as_ref()[self.begin..self.end]
    }

    #[inline]
    pub fn into_string(self) -> String {
        Rc::try_unwrap(self.data).unwrap()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.end - self.begin
    }

    #[inline]
    pub fn get_byte(&self, i: usize) -> u8 {
        self.as_str().as_bytes()[i]
    }

    pub fn slice(&self, begin: isize, end: isize) -> StringObj {
        let self_len = self.len() as isize + 1;
        let bi = begin as usize;
        let ei = ((self_len + end) % self_len) as usize;
        StringObj {
            data: Rc::clone(&self.data),
            begin: bi,
            end: ei,
        }
    }

    pub fn iter(&self) -> StringIter {
        self.as_str().chars()
    }
}

impl Clone for StringObj {
    #[inline]
    fn clone(&self) -> Self {
        StringObj {
            data: Rc::clone(&self.data),
            begin: self.begin,
            end: self.end,
        }
    }
}

impl PartialEq for StringObj {
    #[inline]
    fn eq(&self, other: &StringObj) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl Eq for StringObj {}

impl PartialOrd for StringObj {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StringObj {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        dbg!(self.as_str());
        dbg!(other.as_str());
        self.as_str().cmp(other.as_str())
    }
}

// ----------------------------------------------------------------------------
// MapObj

pub type GosHashMap = HashMap<GosValue, RefCell<GosValue>>;

#[derive(Debug)]
pub struct MapObj {
    pub dark: bool,
    default_val: RefCell<GosValue>,
    map: Rc<RefCell<GosHashMap>>,
}

impl MapObj {
    pub fn new(default_val: GosValue) -> MapObj {
        MapObj {
            dark: false,
            default_val: RefCell::new(default_val),
            map: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// deep_clone creates a new MapObj with duplicated content of 'self.map'
    pub fn deep_clone(&self) -> MapObj {
        MapObj {
            dark: false,
            default_val: self.default_val.clone(),
            map: Rc::new(RefCell::new(self.map.borrow().clone())),
        }
    }

    #[inline]
    pub fn insert(&self, key: GosValue, val: GosValue) -> Option<GosValue> {
        self.map
            .borrow_mut()
            .insert(key, RefCell::new(val))
            .map(|x| x.into_inner())
    }

    #[inline]
    pub fn get(&self, key: &GosValue) -> GosValue {
        let mref = self.map.borrow();
        let cell = match mref.get(key) {
            Some(v) => v,
            None => &self.default_val,
        };
        cell.clone().into_inner()
    }

    /// touch_key makes sure there is a value for the 'key', a default value is set if
    /// the value is empty
    #[inline]
    pub fn touch_key(&self, key: &GosValue) {
        if self.map.borrow().get(&key).is_none() {
            self.map
                .borrow_mut()
                .insert(key.clone(), self.default_val.clone());
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.map.borrow().len()
    }

    #[inline]
    pub fn borrow_data_mut(&self) -> RefMut<GosHashMap> {
        self.map.borrow_mut()
    }

    #[inline]
    pub fn borrow_data(&self) -> Ref<GosHashMap> {
        self.map.borrow()
    }

    #[inline]
    pub fn clone_inner(&self) -> Rc<RefCell<GosHashMap>> {
        Rc::clone(&self.map)
    }
}

impl Clone for MapObj {
    fn clone(&self) -> Self {
        MapObj {
            dark: false,
            default_val: self.default_val.clone(),
            map: Rc::clone(&self.map),
        }
    }
}

impl PartialEq for MapObj {
    fn eq(&self, _other: &MapObj) -> bool {
        unreachable!() //false
    }
}

impl Eq for MapObj {}

// ----------------------------------------------------------------------------
// SliceObj

pub type GosVec = Vec<RefCell<GosValue>>;

#[derive(Debug)]
pub struct SliceObj {
    pub dark: bool,
    begin: usize,
    end: usize,
    soft_cap: usize, // <= self.vec.capacity()
    vec: Rc<RefCell<GosVec>>,
}

impl<'a> SliceObj {
    pub fn new(len: usize, cap: usize, default_val: Option<&GosValue>) -> SliceObj {
        assert!(cap >= len);
        let mut val = SliceObj {
            dark: false,
            begin: 0,
            end: 0,
            soft_cap: cap,
            vec: Rc::new(RefCell::new(Vec::with_capacity(cap))),
        };
        for _ in 0..len {
            val.push(default_val.unwrap().clone());
        }
        val
    }

    pub fn with_data(val: Vec<GosValue>) -> SliceObj {
        SliceObj {
            dark: false,
            begin: 0,
            end: val.len(),
            soft_cap: val.len(),
            vec: Rc::new(RefCell::new(
                val.into_iter().map(|x| RefCell::new(x)).collect(),
            )),
        }
    }

    /// deep_clone creates a new SliceObj with duplicated content of 'self.vec'
    pub fn deep_clone(&self) -> SliceObj {
        let vec = Vec::from_iter(self.vec.borrow()[self.begin..self.end].iter().cloned());
        SliceObj {
            dark: false,
            begin: 0,
            end: self.cap(),
            soft_cap: self.cap(),
            vec: Rc::new(RefCell::new(vec)),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.end - self.begin
    }

    #[inline]
    pub fn cap(&self) -> usize {
        self.soft_cap - self.begin
    }

    #[inline]
    pub fn borrow(&self) -> SliceRef {
        SliceRef::new(self)
    }

    #[inline]
    pub fn borrow_data_mut(&self) -> std::cell::RefMut<GosVec> {
        self.vec.borrow_mut()
    }

    #[inline]
    pub fn borrow_data(&self) -> std::cell::Ref<GosVec> {
        self.vec.borrow()
    }

    #[inline]
    pub fn push(&mut self, val: GosValue) {
        self.try_grow_vec(self.len() + 1);
        self.vec.borrow_mut().push(RefCell::new(val));
        self.end += 1;
    }

    #[inline]
    pub fn append(&mut self, vals: &mut GosVec) {
        let new_len = self.len() + vals.len();
        self.try_grow_vec(new_len);
        self.vec.borrow_mut().append(vals);
        self.end = self.begin + new_len;
    }

    #[inline]
    pub fn get(&self, i: usize) -> Option<GosValue> {
        self.vec
            .borrow()
            .get(self.begin + i)
            .map(|x| x.clone().into_inner())
    }

    #[inline]
    pub fn set(&self, i: usize, val: GosValue) {
        self.vec.borrow()[self.begin + i].replace(val);
    }

    #[inline]
    pub fn slice(&self, begin: isize, end: isize, max: isize) -> SliceObj {
        let self_len = self.len() as isize + 1;
        let self_cap = self.cap() as isize + 1;
        let bi = begin as usize;
        let ei = ((self_len + end) % self_len) as usize;
        let mi = ((self_cap + max) % self_cap) as usize;
        SliceObj {
            dark: false,
            begin: self.begin + bi,
            end: self.begin + ei,
            soft_cap: self.begin + mi,
            vec: Rc::clone(&self.vec),
        }
    }

    fn try_grow_vec(&mut self, len: usize) {
        let mut cap = self.cap();
        assert!(cap >= self.len());
        if cap >= len {
            return;
        }
        while cap < len {
            if cap < 1024 {
                cap *= 2
            } else {
                cap = (cap as f32 * 1.25) as usize
            }
        }
        let data_len = self.len();
        let mut vec = Vec::from_iter(self.vec.borrow()[self.begin..self.end].iter().cloned());
        vec.reserve_exact(cap - vec.len());
        self.vec = Rc::new(RefCell::new(vec));
        self.begin = 0;
        self.end = data_len;
        self.soft_cap = cap;
    }
}

impl Clone for SliceObj {
    fn clone(&self) -> Self {
        SliceObj {
            dark: false,
            begin: self.begin,
            end: self.end,
            soft_cap: self.soft_cap,
            vec: Rc::clone(&self.vec),
        }
    }
}

pub struct SliceRef<'a> {
    vec_ref: Ref<'a, GosVec>,
    begin: usize,
    end: usize,
}

pub type SliceIter<'a> = std::slice::Iter<'a, RefCell<GosValue>>;

pub type SliceEnumIter<'a> = std::iter::Enumerate<std::slice::Iter<'a, RefCell<GosValue>>>;

impl<'a> SliceRef<'a> {
    pub fn new(s: &SliceObj) -> SliceRef {
        SliceRef {
            vec_ref: s.vec.borrow(),
            begin: s.begin,
            end: s.end,
        }
    }

    pub fn iter(&self) -> SliceIter {
        self.vec_ref[self.begin..self.end].iter()
    }

    #[inline]
    pub fn get(&self, i: usize) -> Option<&RefCell<GosValue>> {
        self.vec_ref.get(self.begin + i)
    }
}

impl PartialEq for SliceObj {
    fn eq(&self, _other: &SliceObj) -> bool {
        unreachable!() //false
    }
}

impl Eq for SliceObj {}

// ----------------------------------------------------------------------------
// StructObj

#[derive(Clone, Debug)]
pub struct StructObj {
    pub dark: bool,
    pub meta: GosMetadata,
    pub fields: Vec<GosValue>,
}

impl StructObj {}

// ----------------------------------------------------------------------------
// InterfaceObj

#[derive(Clone, Debug)]
pub struct InterfaceObj {
    pub meta: GosMetadata,
    // the Named object behind the interface
    // mapping from interface's methods to object's methods
    underlying: Option<(GosValue, Rc<Vec<FunctionKey>>)>,
}

impl InterfaceObj {
    pub fn new(
        meta: GosMetadata,
        underlying: Option<(GosValue, Rc<Vec<FunctionKey>>)>,
    ) -> InterfaceObj {
        InterfaceObj {
            meta: meta,
            underlying: underlying,
        }
    }

    #[inline]
    pub fn underlying(&self) -> &Option<(GosValue, Rc<Vec<FunctionKey>>)> {
        &self.underlying
    }

    #[inline]
    pub fn set_underlying(&mut self, named: GosValue, mapping: Rc<Vec<FunctionKey>>) {
        self.underlying = Some((named, mapping));
    }
}

// ----------------------------------------------------------------------------
// ChannelObj

#[derive(Clone, Debug)]
pub struct ChannelObj {}

// ----------------------------------------------------------------------------
// BoxedObj
/// There are two kinds of boxed vars, which is determined by the behavior of
/// copy_semantic. Struct, Slice and Map have true pointers
/// Others don't have true pointers, so a upvalue-like open/close mechanism is needed
#[derive(Debug, Clone)]
pub enum BoxedObj {
    Nil,
    UpVal(UpValue),
    Struct(Rc<RefCell<StructObj>>),
    SliceMember(Rc<SliceObj>, OpIndex),
    StructField(Rc<RefCell<StructObj>>, OpIndex),
    PkgMember(PackageKey, OpIndex),
}

impl BoxedObj {
    #[inline]
    pub fn new_var_up_val(d: ValueDesc) -> BoxedObj {
        BoxedObj::UpVal(UpValue::new(d))
    }

    // supports only Struct for now
    #[inline]
    pub fn new_var_pointer(val: GosValue) -> BoxedObj {
        match val {
            GosValue::Struct(s) => BoxedObj::Struct(s),
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn new_slice_member(slice: &GosValue, index: OpIndex) -> BoxedObj {
        let s = slice.as_slice();
        BoxedObj::SliceMember(s.clone(), index)
    }

    #[inline]
    pub fn new_struct_field(stru: &GosValue, index: OpIndex) -> BoxedObj {
        let s = stru.as_struct();
        BoxedObj::StructField(s.clone(), index)
    }
}

// ----------------------------------------------------------------------------
// ClosureObj

#[derive(Clone, Debug, PartialEq)]
pub struct ValueDesc {
    pub func: FunctionKey,
    pub index: OpIndex,
    pub typ: ValueType,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UpValueState {
    /// Parent CallFrame is still alive, pointing to a local variable
    Open(ValueDesc), // (what func is the var defined, the index of the var)
    // Parent CallFrame is released, pointing to a Boxed value in the global pool
    Closed(GosValue),
}

#[derive(Clone, Debug, PartialEq)]
pub struct UpValue {
    pub inner: Rc<RefCell<UpValueState>>,
}

impl UpValue {
    pub fn new(d: ValueDesc) -> UpValue {
        UpValue {
            inner: Rc::new(RefCell::new(UpValueState::Open(d))),
        }
    }

    pub fn downgrade(&self) -> WeakUpValue {
        WeakUpValue {
            inner: Rc::downgrade(&self.inner),
        }
    }

    pub fn desc(&self) -> ValueDesc {
        let r: &UpValueState = &self.inner.borrow();
        match r {
            UpValueState::Open(d) => d.clone(),
            _ => unreachable!(),
        }
    }

    pub fn close(&self, val: GosValue) {
        *self.inner.borrow_mut() = UpValueState::Closed(val);
    }
}

#[derive(Clone, Debug)]
pub struct WeakUpValue {
    pub inner: Weak<RefCell<UpValueState>>,
}

impl WeakUpValue {
    pub fn upgrade(&self) -> Option<UpValue> {
        Weak::upgrade(&self.inner).map(|x| UpValue { inner: x })
    }
}

/// ClosureObj is a variable containing a pinter to a function and
/// a. a receiver, in which case, it is a bound-method
/// b. upvalues, in which case, it is a "real" closure
///
#[derive(Clone, Debug)]
pub struct ClosureObj {
    pub func: FunctionKey,
    pub receiver: Option<GosValue>,
    upvalues: Option<Vec<UpValue>>,
}

impl ClosureObj {
    pub fn new(
        key: FunctionKey,
        receiver: Option<GosValue>,
        upvalues: Option<Vec<ValueDesc>>,
    ) -> ClosureObj {
        ClosureObj {
            func: key,
            receiver: receiver,
            upvalues: upvalues.map(|uvs| uvs.into_iter().map(|x| UpValue::new(x)).collect()),
        }
    }

    #[inline]
    pub fn has_upvalues(&self) -> bool {
        self.upvalues.is_some()
    }

    #[inline]
    pub fn upvalues(&self) -> &Vec<UpValue> {
        self.upvalues.as_ref().unwrap()
    }
}

// ----------------------------------------------------------------------------
// PackageVal

/// PackageVal is part of the generated Bytecode, it stores imports, consts,
/// vars, funcs declared in a package
#[derive(Clone, Debug)]
pub struct PackageVal {
    name: String,
    members: Vec<GosValue>, // imports, const, var, func are all stored here
    member_indices: HashMap<String, OpIndex>,
    // maps func_member_index of the constructor to pkg_member_index
    var_mapping: Option<HashMap<OpIndex, OpIndex>>,
}

impl PackageVal {
    pub fn new(name: String) -> PackageVal {
        PackageVal {
            name: name,
            members: Vec::new(),
            member_indices: HashMap::new(),
            var_mapping: Some(HashMap::new()),
        }
    }

    pub fn add_member(&mut self, name: String, val: GosValue) -> OpIndex {
        self.members.push(val);
        let index = (self.members.len() - 1) as OpIndex;
        self.member_indices.insert(name, index);
        index as OpIndex
    }

    pub fn add_var_mapping(&mut self, name: String, fn_index: OpIndex) -> OpIndex {
        let index = *self.get_member_index(&name).unwrap();
        self.var_mapping
            .as_mut()
            .unwrap()
            .insert(fn_index.into(), index);
        index
    }

    pub fn var_mut(&mut self, fn_member_index: OpIndex) -> &mut GosValue {
        let index = self.var_mapping.as_ref().unwrap()[&fn_member_index];
        &mut self.members[index as usize]
    }

    pub fn var_count(&self) -> usize {
        self.var_mapping.as_ref().unwrap().len()
    }

    pub fn get_member_index(&self, name: &str) -> Option<&OpIndex> {
        self.member_indices.get(name)
    }

    pub fn inited(&self) -> bool {
        self.var_mapping.is_none()
    }

    pub fn set_inited(&mut self) {
        self.var_mapping = None
    }

    #[inline]
    pub fn member(&self, i: OpIndex) -> &GosValue {
        &self.members[i as usize]
    }

    #[inline]
    pub fn member_mut(&mut self, i: OpIndex) -> &mut GosValue {
        &mut self.members[i as usize]
    }
}

// ----------------------------------------------------------------------------
// FunctionVal

/// EntIndex is for addressing a variable in the scope of a function
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EntIndex {
    Const(OpIndex),
    LocalVar(OpIndex),
    UpValue(OpIndex),
    PackageMember(OpIndex),
    BuiltInVal(Opcode), // built-in identifiers
    BuiltInType(GosMetadata),
    Blank,
}

impl From<EntIndex> for OpIndex {
    fn from(t: EntIndex) -> OpIndex {
        match t {
            EntIndex::Const(i) => i,
            EntIndex::LocalVar(i) => i,
            EntIndex::UpValue(i) => i,
            EntIndex::PackageMember(i) => i,
            EntIndex::BuiltInVal(_) => unreachable!(),
            EntIndex::BuiltInType(_) => unreachable!(),
            EntIndex::Blank => unreachable!(),
        }
    }
}

/// FunctionVal is the direct container of the Opcode.
#[derive(Clone, Debug)]
pub struct FunctionVal {
    pub package: PackageKey,
    pub meta: GosMetadata,
    pub code: Vec<Instruction>,
    pub consts: Vec<GosValue>,
    pub up_ptrs: Vec<ValueDesc>,

    pub ret_zeros: Vec<GosValue>,
    pub local_zeros: Vec<GosValue>,

    param_count: usize,
    entities: HashMap<EntityKey, EntIndex>,
    local_alloc: u16,
    variadic_type: Option<ValueType>,
    is_ctor: bool,
}

impl FunctionVal {
    pub fn new(
        package: PackageKey,
        meta: GosMetadata,
        objs: &VMObjects,
        ctor: bool,
    ) -> FunctionVal {
        match &objs.metas[meta.as_non_ptr()] {
            MetadataType::Signature(s) => {
                let returns: Vec<GosValue> = s.results.iter().map(|x| x.zero_val(objs)).collect();
                let params = s.params.len() + s.recv.map_or(0, |_| 1);
                let vtype = s.variadic.map(|x| x.get_value_type(&objs.metas));
                FunctionVal {
                    package: package,
                    meta: meta,
                    code: Vec::new(),
                    consts: Vec::new(),
                    up_ptrs: Vec::new(),
                    ret_zeros: returns,
                    local_zeros: Vec::new(),
                    param_count: params,
                    entities: HashMap::new(),
                    local_alloc: 0,
                    variadic_type: vtype,
                    is_ctor: ctor,
                }
            }
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn param_count(&self) -> usize {
        self.param_count
    }

    #[inline]
    pub fn ret_count(&self) -> usize {
        self.ret_zeros.len()
    }

    #[inline]
    pub fn is_ctor(&self) -> bool {
        self.is_ctor
    }

    #[inline]
    pub fn variadic(&self) -> Option<ValueType> {
        self.variadic_type
    }

    #[inline]
    pub fn local_count(&self) -> usize {
        self.local_alloc as usize - self.param_count() - self.ret_count()
    }

    #[inline]
    pub fn entity_index(&self, entity: &EntityKey) -> Option<&EntIndex> {
        self.entities.get(entity)
    }

    #[inline]
    pub fn const_val(&self, index: OpIndex) -> &GosValue {
        &self.consts[index as usize]
    }

    #[inline]
    pub fn offset(&self, loc: usize) -> OpIndex {
        // todo: don't crash if OpIndex overflows
        OpIndex::try_from((self.code.len() - loc) as isize).unwrap()
    }

    #[inline]
    pub fn next_code_index(&self) -> usize {
        self.code.len()
    }

    #[inline]
    pub fn emit_inst(
        &mut self,
        op: Opcode,
        type0: Option<ValueType>,
        type1: Option<ValueType>,
        type2: Option<ValueType>,
        imm: Option<i32>,
    ) {
        let i = Instruction::new(op, type0, type1, type2, imm);
        self.code.push(i);
    }

    pub fn emit_raw_inst(&mut self, u: u64) {
        let i = Instruction::from_u64(u);
        self.code.push(i);
    }

    pub fn emit_code_with_type(&mut self, code: Opcode, t: ValueType) {
        self.emit_inst(code, Some(t), None, None, None);
    }

    pub fn emit_code_with_imm(&mut self, code: Opcode, imm: OpIndex) {
        self.emit_inst(code, None, None, None, Some(imm));
    }

    pub fn emit_code_with_type_imm(&mut self, code: Opcode, t: ValueType, imm: OpIndex) {
        self.emit_inst(code, Some(t), None, None, Some(imm));
    }

    pub fn emit_code(&mut self, code: Opcode) {
        self.emit_inst(code, None, None, None, None);
    }

    /// returns the index of the const if it's found
    pub fn get_const_index(&self, val: &GosValue) -> Option<EntIndex> {
        self.consts.iter().enumerate().find_map(|(i, x)| {
            if val == x {
                Some(EntIndex::Const(i as OpIndex))
            } else {
                None
            }
        })
    }

    pub fn add_local(&mut self, entity: Option<EntityKey>) -> EntIndex {
        let result = self.local_alloc as OpIndex;
        if let Some(key) = entity {
            let old = self.entities.insert(key, EntIndex::LocalVar(result));
            assert_eq!(old, None);
        };
        self.local_alloc += 1;
        EntIndex::LocalVar(result)
    }

    pub fn add_local_zero(&mut self, zero: GosValue) {
        self.local_zeros.push(zero)
    }

    /// add a const or get the index of a const.
    /// when 'entity' is no none, it's a const define, so it should not be called with the
    /// same 'entity' more than once
    pub fn add_const(&mut self, entity: Option<EntityKey>, cst: GosValue) -> EntIndex {
        if let Some(index) = self.get_const_index(&cst) {
            index
        } else {
            self.consts.push(cst);
            let result = (self.consts.len() - 1).try_into().unwrap();
            if let Some(key) = entity {
                let old = self.entities.insert(key, EntIndex::Const(result));
                assert_eq!(old, None);
            }
            EntIndex::Const(result)
        }
    }

    pub fn try_add_upvalue(&mut self, entity: &EntityKey, uv: ValueDesc) -> EntIndex {
        self.entities
            .get(entity)
            .map(|x| *x)
            .or_else(|| {
                self.up_ptrs.push(uv);
                let i = (self.up_ptrs.len() - 1).try_into().ok();
                let et = EntIndex::UpValue(i.unwrap());
                self.entities.insert(*entity, et);
                i.map(|x| EntIndex::UpValue(x))
            })
            .unwrap()
    }
}
