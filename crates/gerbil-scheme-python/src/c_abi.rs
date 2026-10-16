// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Thin owner-thread adapters for independent compiled C ABI algorithm exports.
//! No Scheme initialization, source evaluation, algorithm registry or fallback.
use crate::{OwnedValue, ValueLimits};
use num_bigint::{BigInt, Sign};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    ffi::c_void,
    fmt,
    rc::Rc,
};

/// Version shared with `GERBIL_OWNED_ABI_VERSION` in the C header.
pub const ABI_VERSION: u32 = 1;
/// Stable contract identifier; distinct from an artifact or source digest.
pub const ABI_ID: &str = "gerbil-owned/v1";
/// Producer declaration that callbacks do not depend on a Scheme VM, not proof.
pub const NO_VM: u32 = 1;
/// Repository-relative public header defining these exact layouts.
pub const HEADER_PATH: &str = "include/gerbil_scheme_owned.h";
/// Exact header bytes packaged with this crate's compiled ABI definitions.
pub const HEADER_SOURCE: &str = include_str!("../../../include/gerbil_scheme_owned.h");
/// Release exactly one producer allocation on its owner thread.
pub type Release = unsafe extern "C" fn(*mut c_void);
/// Borrow input for one compiled call and transfer one independent owned result.
pub type Call = unsafe extern "C" fn(*mut c_void, *const View, *mut RawValue) -> i32;
/// Snapshot a component without borrowing the component's mutable storage.
pub type Snapshot = unsafe extern "C" fn(*mut c_void, *mut RawValue) -> i32;

/// Copy a borrowed C input using the same typed decoder as owned results.
/// # Safety
/// Every reachable span must be readable and immutable throughout this call.
/// # Errors
/// Rejects null pointers, malformed values and caller structural limits.
pub unsafe fn read_input(input: *const View, limits: ValueLimits) -> Result<OwnedValue, Error> {
    // SAFETY: the caller guarantees readable input when non-null.
    let input = unsafe { input.as_ref() }.ok_or(Error::Invalid("null input"))?;
    // SAFETY: the same caller contract covers all child spans.
    unsafe { decode(input, limits, 0, &mut 0) }
}

/// Transfer one independently allocated transport value to the C result owner.
/// This allocates no callable or component and executes no algorithm.
/// # Errors
/// Reports structural limits before transferring ownership.
pub fn publish_value(value: &OwnedValue) -> Result<RawValue, Error> {
    let storage = Box::new(Encoded::new(value, 0, &mut 0)?);
    let view = storage.view;
    Ok(RawValue { abi_version: ABI_VERSION, flags: NO_VM,
        context: Box::into_raw(storage).cast(), view, release: Some(release_published_value) })
}
unsafe extern "C" fn release_published_value(context: *mut c_void) {
    if !context.is_null() {
        // SAFETY: publish_value transfers this exact allocation and one release right.
        drop(unsafe { Box::from_raw(context.cast::<Encoded>()) });
    }
}

/// Borrowed finite acyclic value layout, valid only within its owner lifetime.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub kind: u32,
    pub flags: u32,
    pub bits: u64,
    pub bytes: *const u8,
    pub length: usize,
    pub children: *const Self,
    pub count: usize,
}
impl Default for View {
    fn default() -> Self {
        Self {
            kind: 0,
            flags: 0,
            bits: 0,
            bytes: std::ptr::null(),
            length: 0,
            children: std::ptr::null(),
            count: 0,
        }
    }
}
/// Single-transfer result allocation with a producer-owned release callback.
#[repr(C)]
#[derive(Debug, Default)]
pub struct RawValue {
    pub abi_version: u32,
    pub flags: u32,
    pub context: *mut c_void,
    pub view: View,
    pub release: Option<Release>,
}
/// Single-transfer captured context and its compiled call/release entries.
#[repr(C)]
#[derive(Debug, Default)]
pub struct RawCallable {
    pub abi_version: u32,
    pub flags: u32,
    pub context: *mut c_void,
    pub call: Option<Call>,
    pub release: Option<Release>,
}
/// Single-transfer component context and its transactional state entries.
#[repr(C)]
#[derive(Debug, Default)]
pub struct RawState {
    pub abi_version: u32,
    pub flags: u32,
    pub context: *mut c_void,
    pub snapshot: Option<Snapshot>,
    pub replace: Option<Call>,
    pub release: Option<Release>,
}

/// Distinguishes rejected ABI/values, closed/busy owners and actual producer status.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    Abi,
    MissingCallback,
    Closed,
    Busy,
    Status(i32),
    Invalid(&'static str),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "compiled C ABI: {self:?}")
    }
}
impl std::error::Error for Error {}
fn header(version: u32, flags: u32) -> Result<(), Error> {
    if version != ABI_VERSION || flags != NO_VM {
        Err(Error::Abi)
    } else {
        Ok(())
    }
}
#[derive(Debug)]
struct Context {
    pointer: *mut c_void,
    release: Option<Release>,
}
impl Drop for Context {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            // SAFETY: the consuming constructor admits this typed, live owner.
            unsafe { release(self.pointer) };
        }
    }
}
#[derive(Debug)]
struct Lease {
    context: RefCell<Option<Context>>,
    busy: Cell<bool>,
}
struct BusyGuard<'a>(&'a Cell<bool>);
impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
impl Lease {
    fn new(pointer: *mut c_void, release: Option<Release>) -> Self {
        Self {
            context: RefCell::new(Some(Context { pointer, release })),
            busy: Cell::new(false),
        }
    }
    fn with<T>(&self, f: impl FnOnce(*mut c_void) -> Result<T, Error>) -> Result<T, Error> {
        if self.busy.replace(true) {
            return Err(Error::Busy);
        }
        let _exit = BusyGuard(&self.busy);
        let context = self.context.try_borrow().map_err(|_| Error::Busy)?;
        f(context.as_ref().ok_or(Error::Closed)?.pointer)
    }
    fn close(&self) -> Result<(), Error> {
        if self.busy.get() {
            return Err(Error::Busy);
        }
        self.context
            .try_borrow_mut()
            .map_err(|_| Error::Busy)?
            .take();
        Ok(())
    }
}

/// The provider owns the entire immutable result until this value is dropped.
/// The Rc marker prevents moving foreign owner-affine state to another thread.
#[derive(Debug)]
pub struct ForeignValue {
    raw: RawValue,
    _owner_thread: std::marker::PhantomData<Rc<()>>,
}
impl Drop for ForeignValue {
    fn drop(&mut self) {
        if let Some(release) = self.raw.release.take() {
            // SAFETY: the admitted provider transfers this independent result.
            unsafe { release(self.raw.context) };
        }
    }
}
impl ForeignValue {
    /// # Safety
    /// Consumes one provider result. All reachable spans must be readable,
    /// aligned and immutable until release, whose ABI is valid even on rejection.
    /// # Errors
    /// Rejects a mismatched ABI declaration or absent release callback.
    pub unsafe fn from_raw(raw: RawValue) -> Result<Self, Error> {
        let value = Self {
            raw,
            _owner_thread: std::marker::PhantomData,
        };
        header(value.raw.abi_version, value.raw.flags)?;
        if value.raw.release.is_none() {
            return Err(Error::MissingCallback);
        }
        Ok(value)
    }
    /// Decode a copy. Unsupported projection leaves the foreign result owned.
    /// # Errors
    /// Reports shape, UTF-8, integer, duplicate-key or structural-limit failures.
    pub fn decode(&self, limits: ValueLimits) -> Result<OwnedValue, Error> {
        // SAFETY: from_raw retains the full provider allocation for this call.
        unsafe { decode(&self.raw.view, limits, 0, &mut 0) }
    }
}
fn output(status: i32, raw: RawValue) -> Result<ForeignValue, Error> {
    if status != 0 {
        // A partially written error output is still owned and must be released.
        drop(ForeignValue {
            raw,
            _owner_thread: std::marker::PhantomData,
        });
        return Err(Error::Status(status));
    }
    // SAFETY: each admitted callback promises a complete owned output on success.
    unsafe { ForeignValue::from_raw(raw) }
}

/// A closure created by the compiled provider, not a Rust implementation of it.
#[derive(Clone, Debug)]
pub struct ForeignCallable {
    lease: Rc<Lease>,
    call: Call,
}
impl ForeignCallable {
    /// Bind a producer's actual compiled closure factory, with typed captures.
    /// # Safety
    /// The factory and every returned callback must satisfy `from_raw`'s contract
    /// and remain live until all values, owners and clones are released.
    /// # Errors
    /// Reports structural limits, producer failure or invalid ABI callbacks.
    pub unsafe fn create(
        factory: unsafe extern "C" fn(*const View, *mut RawCallable) -> i32,
        captures: &OwnedValue,
    ) -> Result<Self, Error> {
        let captures = Encoded::new(captures, 0, &mut 0)?;
        let mut raw = RawCallable::default();
        // SAFETY: the caller admits this factory and captures span this call.
        let status = unsafe { factory(&raw const captures.view, &raw mut raw) };
        if status != 0 {
            drop(Context {
                pointer: raw.context,
                release: raw.release,
            });
            return Err(Error::Status(status));
        }
        // SAFETY: the admitted factory transfers one complete owner on success.
        unsafe { Self::from_raw(raw) }
    }
    /// # Safety
    /// Consumes the context. Function code must outlive all owners and calls;
    /// callbacks must satisfy `gerbil_scheme_owned.h`, never enter a Scheme VM,
    /// never unwind, and run/release only on this owner thread. The provider's
    /// declaration alone is not verification of these requirements.
    /// # Errors
    /// Rejects mismatched ABI declarations or absent call/release callbacks.
    pub unsafe fn from_raw(mut raw: RawCallable) -> Result<Self, Error> {
        let release = raw.release.take();
        let lease = Rc::new(Lease::new(raw.context, release));
        header(raw.abi_version, raw.flags)?;
        if release.is_none() {
            return Err(Error::MissingCallback);
        }
        Ok(Self {
            lease,
            call: raw.call.ok_or(Error::MissingCallback)?,
        })
    }
    /// # Errors
    /// Reports closed/reentrant owners, input limits or producer/output failure.
    pub fn call(&self, input: &OwnedValue) -> Result<ForeignValue, Error> {
        let input = Encoded::new(input, 0, &mut 0)?;
        self.lease.with(|context| {
            let mut result = RawValue::default();
            // SAFETY: the lease retains this provider; input storage spans the call.
            let status = unsafe { (self.call)(context, &raw const input.view, &raw mut result) };
            output(status, result)
        })
    }
    /// # Errors
    /// Rejects closing an owner during a callback. Repeated close is harmless.
    pub fn close(&self) -> Result<(), Error> {
        self.lease.close()
    }
}
/// Provider-owned component state. The host never implements its transition.
#[derive(Clone, Debug)]
pub struct ForeignState {
    lease: Rc<Lease>,
    snapshot: Snapshot,
    replace: Call,
}
impl ForeignState {
    /// # Safety
    /// Same factory/code/owner requirements as `ForeignCallable::create`, and the
    /// state transition contract of `ForeignState::from_raw`.
    /// # Errors
    /// Reports structural limits, producer failure or invalid ABI callbacks.
    pub unsafe fn create(
        factory: unsafe extern "C" fn(*const View, *mut RawState) -> i32,
        initial: &OwnedValue,
    ) -> Result<Self, Error> {
        let initial = Encoded::new(initial, 0, &mut 0)?;
        let mut raw = RawState::default();
        // SAFETY: the caller admits this factory and initial data spans this call.
        let status = unsafe { factory(&raw const initial.view, &raw mut raw) };
        if status != 0 {
            drop(Context {
                pointer: raw.context,
                release: raw.release,
            });
            return Err(Error::Status(status));
        }
        // SAFETY: the admitted factory transfers one complete owner on success.
        unsafe { Self::from_raw(raw) }
    }
    /// # Safety
    /// Same owner/code/no-VM requirements as `ForeignCallable::from_raw`. Snapshot
    /// results must be independent; replace is transactional on every failure.
    /// # Errors
    /// Rejects mismatched ABI declarations or absent lifecycle/state callbacks.
    pub unsafe fn from_raw(mut raw: RawState) -> Result<Self, Error> {
        let release = raw.release.take();
        let lease = Rc::new(Lease::new(raw.context, release));
        header(raw.abi_version, raw.flags)?;
        if release.is_none() {
            return Err(Error::MissingCallback);
        }
        Ok(Self {
            lease,
            snapshot: raw.snapshot.ok_or(Error::MissingCallback)?,
            replace: raw.replace.ok_or(Error::MissingCallback)?,
        })
    }
    /// # Errors
    /// Reports closed/reentrant owners or producer/output failure.
    pub fn snapshot(&self) -> Result<ForeignValue, Error> {
        self.lease.with(|context| {
            let mut result = RawValue::default();
            // SAFETY: this owner lease retains the admitted snapshot entry.
            output(unsafe { (self.snapshot)(context, &raw mut result) }, result)
        })
    }
    /// # Errors
    /// Reports closed/reentrant owners, input limits or producer/output failure.
    pub fn replace(&self, value: &OwnedValue) -> Result<ForeignValue, Error> {
        let value = Encoded::new(value, 0, &mut 0)?;
        self.lease.with(|context| {
            let mut result = RawValue::default();
            // SAFETY: this owner retains the callback and all borrowed input data.
            output(
                unsafe { (self.replace)(context, &raw const value.view, &raw mut result) },
                result,
            )
        })
    }
    /// # Errors
    /// Rejects closing an owner during a callback. Repeated close is harmless.
    pub fn close(&self) -> Result<(), Error> {
        self.lease.close()
    }
}

#[derive(Debug)]
struct Encoded {
    view: View,
    _bytes: Vec<u8>,
    _children: Vec<Self>,
    _views: Vec<View>,
}
impl Encoded {
    fn new(value: &OwnedValue, depth: usize, nodes: &mut usize) -> Result<Self, Error> {
        let limits = ValueLimits::default();
        if depth > limits.depth || *nodes >= limits.nodes {
            return Err(Error::Invalid("input structure bound"));
        }
        *nodes += 1;
        let mut view = View::default();
        let mut bytes = Vec::new();
        let mut children = Vec::new();
        match value {
            OwnedValue::Null => (),
            OwnedValue::Boolean(v) => {
                view.kind = 1;
                view.bits = u64::from(*v);
            }
            OwnedValue::Integer(v) => {
                view.kind = 2;
                let (sign, magnitude) = v.to_bytes_le();
                bytes = magnitude;
                if sign == Sign::NoSign {
                    bytes.clear();
                }
                view.flags = u32::from(sign == Sign::Minus);
            }
            OwnedValue::Real(v) => {
                view.kind = 3;
                view.bits = v.to_bits();
            }
            OwnedValue::Text(v) => {
                view.kind = 4;
                bytes = v.as_bytes().to_vec();
            }
            OwnedValue::Bytes(v) => {
                view.kind = 5;
                bytes.clone_from(v);
            }
            OwnedValue::List(values) => {
                view.kind = 6;
                children = values
                    .iter()
                    .map(|v| Self::new(v, depth + 1, nodes))
                    .collect::<Result<Vec<_>, _>>()?;
            }
            OwnedValue::Map(values) => {
                view.kind = 7;
                children = Self::fields(values, depth + 1, nodes)?;
            }
        }
        let views = children.iter().map(|v| v.view).collect::<Vec<_>>();
        view.bytes = bytes.as_ptr();
        view.length = bytes.len();
        view.children = views.as_ptr();
        view.count = views.len();
        Ok(Self {
            view,
            _bytes: bytes,
            _children: children,
            _views: views,
        })
    }
    fn fields(
        values: &BTreeMap<String, OwnedValue>,
        depth: usize,
        nodes: &mut usize,
    ) -> Result<Vec<Self>, Error> {
        let mut children = Vec::new();
        for (key, value) in values {
            children.push(Self::new(&OwnedValue::Text(key.clone()), depth, nodes)?);
            children.push(Self::new(value, depth, nodes)?);
        }
        Ok(children)
    }
}
unsafe fn span<'a, T>(pointer: *const T, length: usize) -> Result<&'a [T], Error> {
    if length == 0 {
        return Ok(&[]);
    }
    if pointer.is_null()
        || (pointer as usize) % std::mem::align_of::<T>() != 0
        || length > isize::MAX as usize / std::mem::size_of::<T>()
    {
        return Err(Error::Invalid("foreign span"));
    }
    // SAFETY: the provider constructor contract supplies a readable live span;
    // nullness, alignment and addressable byte length were independently checked.
    Ok(unsafe { std::slice::from_raw_parts(pointer, length) })
}
unsafe fn decode(
    view: &View,
    limits: ValueLimits,
    depth: usize,
    nodes: &mut usize,
) -> Result<OwnedValue, Error> {
    if depth > limits.depth || *nodes >= limits.nodes {
        return Err(Error::Invalid("output structure bound"));
    }
    *nodes += 1;
    if (view.kind != 2 && view.flags != 0) || view.flags > 1 {
        return Err(Error::Invalid("value flags"));
    }
    if view.kind > 7 {
        return Err(Error::Invalid("unsupported value kind"));
    }
    if (view.kind < 2 || view.kind == 3 || view.kind >= 6) && view.length != 0
        || view.kind < 6 && view.count != 0
    {
        return Err(Error::Invalid("unexpected value span"));
    }
    // SAFETY: recursive views remain in the immutable allocation owned by ForeignValue.
    let bytes = unsafe { span(view.bytes, view.length)? };
    Ok(match view.kind {
        0 if view.bits == 0 => OwnedValue::Null,
        1 if view.bits <= 1 => OwnedValue::Boolean(view.bits == 1),
        2 if view.bits == 0
            && bytes.last() != Some(&0)
            && !(bytes.is_empty() && view.flags == 1) =>
        {
            OwnedValue::Integer(BigInt::from_bytes_le(
                if view.flags == 1 {
                    Sign::Minus
                } else {
                    Sign::Plus
                },
                bytes,
            ))
        }
        3 => OwnedValue::Real(f64::from_bits(view.bits)),
        4 if view.bits == 0 => OwnedValue::Text(
            std::str::from_utf8(bytes)
                .map_err(|_| Error::Invalid("UTF-8"))?
                .into(),
        ),
        5 if view.bits == 0 => OwnedValue::Bytes(bytes.to_vec()),
        6 | 7 if view.bits == 0 => {
            if view.count > limits.nodes.saturating_sub(*nodes) {
                return Err(Error::Invalid("output structure bound"));
            }
            // SAFETY: same live immutable provider allocation and checked span shape.
            let children = unsafe { span(view.children, view.count)? };
            let values = children
                .iter()
                .map(|v| unsafe { decode(v, limits, depth + 1, nodes) })
                .collect::<Result<Vec<_>, _>>()?;
            if view.kind == 6 {
                OwnedValue::List(values)
            } else {
                if values.len() % 2 != 0 {
                    return Err(Error::Invalid("map arity"));
                }
                let mut map = BTreeMap::new();
                let mut values = values.into_iter();
                while let Some(key) = values.next() {
                    let OwnedValue::Text(key) = key else {
                        return Err(Error::Invalid("map key type"));
                    };
                    if map
                        .insert(key, values.next().ok_or(Error::Invalid("map arity"))?)
                        .is_some()
                    {
                        return Err(Error::Invalid("duplicate map key"));
                    }
                }
                OwnedValue::Map(map)
            }
        }
        _ => return Err(Error::Invalid("value encoding")),
    })
}
