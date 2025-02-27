use crate::ffi;
use std::ffi::{c_char, c_int};

pub type LuaStateRaw = *mut ffi::lua_State;

#[derive(PartialEq)]
pub struct LuaThread(pub LuaStateRaw);

unsafe impl Send for LuaThread {}

impl LuaThread {
    pub fn new(l: LuaStateRaw) -> Self {
        LuaThread(l)
    }
}

#[derive(PartialEq)]
pub struct LuaState(pub LuaStateRaw);

unsafe impl Send for LuaState {}

impl LuaState {
    pub fn new(l: LuaStateRaw) -> Self {
        LuaState(l)
    }
}

impl Drop for LuaState {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() {
                ffi::lua_close(self.0);
            }
        }
    }
}

pub extern "C-unwind" fn lua_null_function(_: LuaStateRaw) -> c_int {
    0
}

pub struct LuaScopePop {
    state: LuaStateRaw,
}

impl LuaScopePop {
    pub fn new(state: LuaStateRaw) -> Self {
        LuaScopePop { state }
    }
}

impl Drop for LuaScopePop {
    fn drop(&mut self) {
        unsafe {
            ffi::lua_pop(self.state, 1);
        }
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C-unwind" fn lua_traceback(state: LuaStateRaw) -> c_int {
    unsafe {
        let msg = ffi::lua_tostring(state, 1);
        if !msg.is_null() {
            ffi::luaL_traceback(state, state, msg, 1);
        } else {
            ffi::lua_pushliteral(state, "(no error message)");
        }
        1
    }
}

pub trait LuaValue {
    fn from_lua_check(state: LuaStateRaw, index: i32) -> Self;

    fn from_lua(state: LuaStateRaw, index: i32) -> Self;

    fn from_lua_opt(state: LuaStateRaw, index: i32) -> Option<Self>
    where
        Self: Sized;

    fn push_lua(state: LuaStateRaw, v: Self);
}

impl LuaValue for bool {
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_check(state: LuaStateRaw, index: i32) -> bool {
        unsafe {
            ffi::luaL_checktype(state, index, ffi::LUA_TBOOLEAN);
            ffi::lua_toboolean(state, index) != 0
        }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua(state: LuaStateRaw, index: i32) -> bool {
        unsafe { ffi::lua_toboolean(state, index) != 0 }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_opt(state: LuaStateRaw, index: i32) -> Option<bool> {
        unsafe {
            if ffi::lua_isnoneornil(state, index) != 0 {
                None
            } else {
                Some(ffi::lua_toboolean(state, index) != 0)
            }
        }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn push_lua(state: LuaStateRaw, v: bool) {
        unsafe {
            ffi::lua_pushboolean(state, v as c_int);
        }
    }
}

impl LuaValue for i8 {
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_check(state: LuaStateRaw, index: i32) -> i8 {
        unsafe { ffi::luaL_checkinteger(state, index) as i8 }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua(state: LuaStateRaw, index: i32) -> i8 {
        unsafe { ffi::lua_tointeger(state, index) as i8 }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_opt(state: LuaStateRaw, index: i32) -> Option<i8> {
        if unsafe { ffi::lua_isinteger(state, index) } == 0 {
            None
        } else {
            Some(unsafe { ffi::lua_tointeger(state, index) as i8 })
        }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn push_lua(state: LuaStateRaw, v: i8) {
        unsafe {
            ffi::lua_pushinteger(state, v as ffi::lua_Integer);
        }
    }
}

impl LuaValue for u8 {
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_check(state: LuaStateRaw, index: i32) -> u8 {
        unsafe { ffi::luaL_checkinteger(state, index) as u8 }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua(state: LuaStateRaw, index: i32) -> u8 {
        unsafe { ffi::lua_tointeger(state, index) as u8 }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_opt(state: LuaStateRaw, index: i32) -> Option<u8> {
        if unsafe { ffi::lua_isinteger(state, index) } == 0 {
            None
        } else {
            Some(unsafe { ffi::lua_tointeger(state, index) as u8 })
        }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn push_lua(state: LuaStateRaw, v: u8) {
        unsafe {
            ffi::lua_pushinteger(state, v as ffi::lua_Integer);
        }
    }
}

impl LuaValue for i32 {
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_check(state: LuaStateRaw, index: i32) -> i32 {
        unsafe { ffi::luaL_checkinteger(state, index) as i32 }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua(state: LuaStateRaw, index: i32) -> i32 {
        unsafe { ffi::lua_tointeger(state, index) as i32 }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_opt(state: LuaStateRaw, index: i32) -> Option<i32> {
        if unsafe { ffi::lua_isinteger(state, index) } == 0 {
            None
        } else {
            Some(unsafe { ffi::lua_tointeger(state, index) as i32 })
        }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn push_lua(state: LuaStateRaw, v: i32) {
        unsafe {
            ffi::lua_pushinteger(state, v as ffi::lua_Integer);
        }
    }
}

impl LuaValue for u32 {
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_check(state: LuaStateRaw, index: i32) -> u32 {
        unsafe { ffi::luaL_checkinteger(state, index) as u32 }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua(state: LuaStateRaw, index: i32) -> u32 {
        unsafe { ffi::lua_tointeger(state, index) as u32 }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_opt(state: LuaStateRaw, index: i32) -> Option<u32> {
        if unsafe { ffi::lua_isinteger(state, index) } == 0 {
            None
        } else {
            Some(unsafe { ffi::lua_tointeger(state, index) as u32 })
        }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn push_lua(state: LuaStateRaw, v: u32) {
        unsafe {
            ffi::lua_pushinteger(state, v as ffi::lua_Integer);
        }
    }
}

impl LuaValue for usize {
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_check(state: LuaStateRaw, index: i32) -> usize {
        unsafe { ffi::luaL_checkinteger(state, index) as usize }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua(state: LuaStateRaw, index: i32) -> usize {
        unsafe { ffi::lua_tointeger(state, index) as usize }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_opt(state: LuaStateRaw, index: i32) -> Option<usize> {
        if unsafe { ffi::lua_isinteger(state, index) } == 0 {
            None
        } else {
            Some(unsafe { ffi::lua_tointeger(state, index) as usize })
        }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn push_lua(state: LuaStateRaw, v: usize) {
        unsafe {
            ffi::lua_pushinteger(state, v as ffi::lua_Integer);
        }
    }
}

impl LuaValue for isize {
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_check(state: LuaStateRaw, index: i32) -> isize {
        unsafe { ffi::luaL_checkinteger(state, index) as isize }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua(state: LuaStateRaw, index: i32) -> isize {
        unsafe { ffi::lua_tointeger(state, index) as isize }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_opt(state: LuaStateRaw, index: i32) -> Option<isize> {
        if unsafe { ffi::lua_isinteger(state, index) } == 0 {
            None
        } else {
            Some(unsafe { ffi::lua_tointeger(state, index) as isize })
        }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn push_lua(state: LuaStateRaw, v: isize) {
        unsafe {
            ffi::lua_pushinteger(state, v as ffi::lua_Integer);
        }
    }
}

impl LuaValue for i64 {
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_check(state: LuaStateRaw, index: i32) -> i64 {
        unsafe { ffi::luaL_checkinteger(state, index) as i64 }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua(state: LuaStateRaw, index: i32) -> i64 {
        unsafe { ffi::lua_tointeger(state, index) as i64 }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_opt(state: LuaStateRaw, index: i32) -> Option<i64> {
        if unsafe { ffi::lua_isinteger(state, index) } == 0 {
            None
        } else {
            Some(unsafe { ffi::lua_tointeger(state, index) as i64 })
        }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn push_lua(state: LuaStateRaw, v: i64) {
        unsafe {
            ffi::lua_pushinteger(state, v as ffi::lua_Integer);
        }
    }
}

impl LuaValue for u64 {
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_check(state: LuaStateRaw, index: i32) -> u64 {
        unsafe { ffi::luaL_checkinteger(state, index) as u64 }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua(state: LuaStateRaw, index: i32) -> u64 {
        unsafe { ffi::lua_tointeger(state, index) as u64 }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_opt(state: LuaStateRaw, index: i32) -> Option<u64> {
        if unsafe { ffi::lua_isinteger(state, index) } == 0 {
            None
        } else {
            Some(unsafe { ffi::lua_tointeger(state, index) as u64 })
        }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn push_lua(state: LuaStateRaw, v: u64) {
        unsafe {
            ffi::lua_pushinteger(state, v as ffi::lua_Integer);
        }
    }
}

impl LuaValue for f64 {
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_check(state: LuaStateRaw, index: i32) -> f64 {
        unsafe { ffi::luaL_checknumber(state, index) as f64 }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua(state: LuaStateRaw, index: i32) -> f64 {
        unsafe { ffi::lua_tonumber(state, index) as f64 }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_opt(state: LuaStateRaw, index: i32) -> Option<f64> {
        if unsafe { ffi::lua_isnumber(state, index) } == 0 {
            None
        } else {
            Some(unsafe { ffi::lua_tonumber(state, index) as f64 })
        }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn push_lua(state: LuaStateRaw, v: f64) {
        unsafe {
            ffi::lua_pushnumber(state, v as ffi::lua_Number);
        }
    }
}

impl LuaValue for &[u8] {
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_check(state: LuaStateRaw, index: i32) -> &'static [u8] {
        unsafe {
            let mut len = 0;
            let ptr = ffi::luaL_checklstring(state, index, &mut len);
            std::slice::from_raw_parts(ptr as *const u8, len)
        }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua(state: LuaStateRaw, index: i32) -> &'static [u8] {
        unsafe {
            let mut len = 0;
            let ptr = ffi::lua_tolstring(state, index, &mut len);
            std::slice::from_raw_parts(ptr as *const u8, len)
        }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_opt(state: LuaStateRaw, index: i32) -> Option<&'static [u8]> {
        unsafe {
            if ffi::lua_isnoneornil(state, index) != 0 {
                None
            } else {
                let mut len = 0;
                let ptr = ffi::luaL_checklstring(state, index, &mut len);
                Some(std::slice::from_raw_parts(ptr as *const u8, len))
            }
        }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn push_lua(state: LuaStateRaw, v: &[u8]) {
        unsafe {
            ffi::lua_pushlstring(state, v.as_ptr() as *const c_char, v.len());
        }
    }
}

impl LuaValue for &str {
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_check(state: LuaStateRaw, index: i32) -> &'static str {
        unsafe {
            let mut len = 0;
            let ptr = ffi::luaL_checklstring(state, index, &mut len);
            let slice = std::slice::from_raw_parts(ptr as *const u8, len);
            std::str::from_utf8_unchecked(slice)
        }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua(state: LuaStateRaw, index: i32) -> &'static str {
        unsafe {
            let mut len = 0;
            let ptr = ffi::lua_tolstring(state, index, &mut len);
            let slice = std::slice::from_raw_parts(ptr as *const u8, len);
            std::str::from_utf8_unchecked(slice)
        }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_opt(state: LuaStateRaw, index: i32) -> Option<&'static str> {
        unsafe {
            if ffi::lua_isnoneornil(state, index) != 0 {
                None
            } else {
                let mut len = 0;
                let ptr = ffi::luaL_checklstring(state, index, &mut len);
                let slice = std::slice::from_raw_parts(ptr as *const u8, len);
                Some(std::str::from_utf8_unchecked(slice))
            }
        }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn push_lua(state: LuaStateRaw, v: &str) {
        unsafe {
            ffi::lua_pushlstring(state, v.as_ptr() as *const c_char, v.len());
        }
    }
}

impl LuaValue for String {
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_check(state: LuaStateRaw, index: i32) -> String {
        unsafe {
            let mut len = 0;
            let ptr = ffi::luaL_checklstring(state, index, &mut len);
            let slice = std::slice::from_raw_parts(ptr as *const u8, len);
            String::from_utf8_lossy(slice).into_owned()
        }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua(state: LuaStateRaw, index: i32) -> String {
        unsafe {
            let mut len = 0;
            let ptr = ffi::lua_tolstring(state, index, &mut len);
            let slice = std::slice::from_raw_parts(ptr as *const u8, len);
            String::from_utf8_lossy(slice).into_owned()
        }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_lua_opt(state: LuaStateRaw, index: i32) -> Option<String> {
        unsafe {
            if ffi::lua_isnoneornil(state, index) != 0 {
                None
            } else {
                let mut len = 0;
                let ptr = ffi::luaL_checklstring(state, index, &mut len);
                let slice = std::slice::from_raw_parts(ptr as *const u8, len);
                Some(String::from_utf8_lossy(slice).into_owned())
            }
        }
    }

    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn push_lua(state: LuaStateRaw, v: String) {
        unsafe {
            ffi::lua_pushlstring(state, v.as_ptr() as *const c_char, v.len());
        }
    }
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn opt_field<T>(state: LuaStateRaw, mut index: i32, field: &str) -> Option<T>
where
    T: LuaValue,
{
    if index < 0 {
        unsafe {
            index = ffi::lua_gettop(state) + index + 1;
        }
    }

    let _scope = LuaScopePop::new(state);
    unsafe {
        ffi::lua_pushlstring(state, field.as_ptr() as *const c_char, field.len());
        if ffi::lua_rawget(state, index) <= ffi::LUA_TNIL {
            return None;
        }
    }

    LuaValue::from_lua_opt(state, -1)
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn lua_get<T>(state: LuaStateRaw, index: i32) -> T
where
    T: LuaValue,
{
    LuaValue::from_lua_check(state, index)
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn lua_to<T>(state: LuaStateRaw, index: i32) -> T
where
    T: LuaValue,
{
    LuaValue::from_lua(state, index)
}

#[inline]
pub fn lua_opt<T>(state: LuaStateRaw, index: i32) -> Option<T>
where
    T: LuaValue,
{
    LuaValue::from_lua_opt(state, index)
}

#[inline]
pub fn lua_push<T>(state: LuaStateRaw, v: T)
where
    T: LuaValue,
{
    LuaValue::push_lua(state, v);
}

#[derive(PartialEq, Eq)]
pub enum LuaType {
    Nil,
    Boolean,
    LightUserData,
    Number,
    String,
    Table,
    Function,
    UserData,
    Thread,
}

impl From<LuaType> for i32 {
    fn from(lua_type: LuaType) -> Self {
        match lua_type {
            LuaType::Nil => 0,
            LuaType::Boolean => 1,
            LuaType::LightUserData => 2,
            LuaType::Number => 3,
            LuaType::String => 4,
            LuaType::Table => 5,
            LuaType::Function => 6,
            LuaType::UserData => 7,
            LuaType::Thread => 8,
        }
    }
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn lua_type(state: LuaStateRaw, index: i32) -> LuaType {
    let ltype = unsafe { ffi::lua_type(state, index) };
    match ltype {
        ffi::LUA_TNIL => LuaType::Nil,
        ffi::LUA_TBOOLEAN => LuaType::Boolean,
        ffi::LUA_TLIGHTUSERDATA => LuaType::LightUserData,
        ffi::LUA_TNUMBER => LuaType::Number,
        ffi::LUA_TSTRING => LuaType::String,
        ffi::LUA_TTABLE => LuaType::Table,
        ffi::LUA_TFUNCTION => LuaType::Function,
        ffi::LUA_TUSERDATA => LuaType::UserData,
        ffi::LUA_TTHREAD => LuaType::Thread,
        _ => unreachable!(),
    }
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn lua_error(state: LuaStateRaw, message: &str) -> ! {
    unsafe {
        ffi::lua_pushlstring(state, message.as_ptr() as *const c_char, message.len());
        ffi::lua_error(state)
    }
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn throw_error(state: LuaStateRaw) -> ! {
    unsafe { ffi::lua_error(state) }
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn type_name(state: LuaStateRaw, ltype: i32) -> &'static str {
    unsafe {
        std::ffi::CStr::from_ptr(ffi::lua_typename(state, ltype))
            .to_str()
            .unwrap_or_default()
    }
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn lua_pushnil(state: LuaStateRaw) {
    unsafe {
        ffi::lua_pushnil(state);
    }
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn is_integer(state: LuaStateRaw, index: i32) -> bool {
    unsafe { ffi::lua_isinteger(state, index) != 0 }
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn lua_top(state: LuaStateRaw) -> i32 {
    unsafe { ffi::lua_gettop(state) }
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn lua_settop(state: LuaStateRaw, idx: i32) {
    unsafe {
        ffi::lua_settop(state, idx);
    }
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn lua_pop(state: LuaStateRaw, n: i32) {
    unsafe {
        ffi::lua_pop(state, n);
    }
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn lua_rawget(state: LuaStateRaw, index: i32) -> i32 {
    unsafe { ffi::lua_rawget(state, index) }
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn lua_rawlen(state: LuaStateRaw, index: i32) -> usize {
    unsafe { ffi::lua_rawlen(state, index) }
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn lua_rawgeti(state: LuaStateRaw, index: i32, n: usize) {
    unsafe {
        ffi::lua_rawgeti(state, index, n as ffi::lua_Integer);
    }
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn lua_next(state: LuaStateRaw, index: i32) -> bool {
    unsafe { ffi::lua_next(state, index) != 0 }
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn lua_checktype(state: LuaStateRaw, index: i32, ltype: i32) {
    unsafe {
        ffi::luaL_checktype(state, index, ltype);
    }
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn luaL_checkstack(state: LuaStateRaw, sz: i32, msg: *const c_char) {
    unsafe {
        ffi::luaL_checkstack(state, sz, msg);
    }
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn push_c_string(state: LuaStateRaw, s: *const i8) {
    unsafe {
        ffi::lua_pushstring(state, s);
    }
}

///stack +1
#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn lua_as_str(state: LuaStateRaw, index: i32) -> &'static str {
    unsafe {
        let mut len = 0;
        let ptr = ffi::luaL_tolstring(state, index, &mut len);
        let slice = std::slice::from_raw_parts(ptr as *const u8, len);
        std::str::from_utf8_unchecked(slice)
    }
}

///stack +1
#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn lua_as_slice(state: LuaStateRaw, index: i32) -> &'static [u8] {
    unsafe {
        let mut len = 0;
        let ptr = ffi::luaL_tolstring(state, index, &mut len);
        std::slice::from_raw_parts(ptr as *const u8, len)
    }
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn lua_pushlightuserdata(state: LuaStateRaw, p: *mut std::ffi::c_void) {
    unsafe {
        ffi::lua_pushlightuserdata(state, p);
    }
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn to_string_unchecked(state: *mut ffi::lua_State, index: i32) -> String {
    match lua_type(state, index) {
        LuaType::Nil => String::from("nil"),
        LuaType::Boolean => {
            if lua_to::<bool>(state, index) {
                String::from("true")
            } else {
                String::from("false")
            }
        }
        LuaType::Number => {
            if is_integer(state, index) {
                lua_to::<i64>(state, index).to_string()
            } else {
                lua_to::<f64>(state, index).to_string()
            }
        }
        LuaType::String => lua_as_str(state, index).to_string(),
        _ => String::from("string type expected"),
    }
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn lua_newuserdata<T>(
    state: *mut ffi::lua_State,
    val: T,
    metaname: *const c_char,
    lib: &[ffi::luaL_Reg],
) -> Option<&T> {
    extern "C-unwind" fn lua_dropuserdata<T>(state: *mut ffi::lua_State) -> c_int {
        unsafe {
            let p = ffi::lua_touserdata(state, 1);
            if p.is_null() {
                return 0;
            }
            let p = p as *mut T;
            std::ptr::drop_in_place(p);
        }
        0
    }

    unsafe {
        let ptr = ffi::lua_newuserdatauv(state, std::mem::size_of::<T>(), 0) as *mut T;
        let ptr = std::ptr::NonNull::new(ptr)?;

        ptr.as_ptr().write(val);

        if ffi::luaL_newmetatable(state, metaname) != 0 {
            ffi::lua_createtable(state, 0, lib.len() as c_int);
            ffi::luaL_setfuncs(state, lib.as_ptr(), 0);
            ffi::lua_setfield(state, -2, cstr!("__index"));
            ffi::lua_pushcfunction(state, lua_dropuserdata::<T>);
            ffi::lua_setfield(state, -2, cstr!("__gc"));
        }

        ffi::lua_setmetatable(state, -2);
        Some(&*ptr.as_ptr())
    }
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn lua_touserdata<T>(state: *mut ffi::lua_State, index: i32) -> Option<&'static mut T> {
    unsafe {
        let ptr = ffi::lua_touserdata(state, index);
        let ptr = std::ptr::NonNull::new(ptr)?;
        let ptr = ptr.as_ptr() as *mut T;
        Some(&mut *ptr)
    }
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn lua_isinteger(state: LuaStateRaw, index: i32) -> bool {
    unsafe { ffi::lua_isinteger(state, index) != 0 }
}

#[inline]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn lua_from_raw_parts(state: LuaStateRaw, mut index: i32) -> &'static [u8] {
    unsafe {
        if index < 0 {
            index = ffi::lua_gettop(state) + index + 1;
        }
        ffi::luaL_checktype(state, index, ffi::LUA_TLIGHTUSERDATA);
        let ptr = ffi::lua_touserdata(state, 1);
        let len = lua_get(state, index + 1);
        std::slice::from_raw_parts(ptr as *const u8, len)
    }
}

/// Converts an `isize` value from Lua state at the given index into a Rust `T` object.
/// 
/// # Arguments
/// 
/// * `state` - The Lua state.
/// * `index` - The index in the Lua stack.
/// 
/// # Safety
/// 
/// This function is unsafe because it dereferences a raw pointer.
/// 
/// # Returns
/// 
/// A `Box<T>` containing the Rust object.
pub fn lua_into_userdata<T>(state: LuaStateRaw, index: i32) -> Box<T> {
    let p_as_isize: isize = lua_get(state, index);
    unsafe { Box::from_raw(p_as_isize as *mut T) }
}

pub struct LuaTable {
    state: LuaStateRaw,
    index: i32,
}

impl LuaTable {
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn new(state: LuaStateRaw, narr: usize, nrec: usize) -> Self {
        unsafe {
            ffi::lua_createtable(state, narr as i32, nrec as i32);
            LuaTable {
                state,
                index: ffi::lua_gettop(state),
            }
        }
    }

    pub fn from_raw(state: LuaStateRaw, index: i32) -> Self {
        LuaTable { state, index }
    }

    pub fn len(&self) -> usize {
        lua_rawlen(self.state, self.index)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn seti(&self, n: i64) {
        unsafe {
            ffi::lua_seti(self.state, self.index, n);
        }
    }

    pub fn new_table<K>(&self, key: K, narr: usize, nrec: usize) -> LuaTable
    where K: LuaValue {
        unsafe {
            K::push_lua(self.state, key);
            ffi::lua_createtable(self.state, narr as i32, nrec as i32);
            LuaTable {
                state: self.state,
                index: ffi::lua_gettop(self.state),
            }
        }
    }

    pub fn rawset(&self) {
        unsafe {
            ffi::lua_rawset(self.state, self.index);
        }
    }

    pub fn set<K,V>(&self, key: K, val: V)
    where
        K : LuaValue,
        V: LuaValue,
    {
        unsafe {
            K::push_lua(self.state, key);
            V::push_lua(self.state, val);
            ffi::lua_rawset(self.state, self.index);
        }
    }

    pub fn foreach<F>(&self, mut f: F)
    where
        F: FnMut(i32, i32),
    {
        unsafe {
            ffi::lua_pushnil(self.state);
            while ffi::lua_next(self.state, self.index) != 0 {
                f(-2, -1);
                ffi::lua_pop(self.state, 1);
            }
        }
    }
}
