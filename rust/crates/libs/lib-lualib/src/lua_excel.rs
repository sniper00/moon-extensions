use calamine::{open_workbook, Data, Reader, Xlsx};
use csv::ReaderBuilder;
use lib_lua::{
    self, cstr,
    ffi::{self, luaL_Reg},
    laux::{self, LuaState}, lreg, lreg_null, luaL_newlib,
};
use std::{os::raw::c_int, path::Path};

fn read_csv(state: LuaState, path: &Path, max_row: usize) -> c_int {
    let res = ReaderBuilder::new().has_headers(false).from_path(path);
    unsafe {
        ffi::lua_createtable(state, 0, 0);
    }

    match res {
        Ok(mut reader) => {
            unsafe {
                ffi::lua_createtable(state, 0, 2);
                laux::lua_push(
                    state,
                    path.file_stem()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or_default(),
                );
                ffi::lua_setfield(state, -2, cstr!("sheet_name"));
                ffi::lua_createtable(state, 1024, 0);
            }

            let mut idx: usize = 0;

            for result in reader.records() {
                if idx >= max_row {
                    break;
                }
                match result {
                    Ok(record) => unsafe {
                        ffi::lua_createtable(state, 0, record.len() as i32);
                        for (i, field) in record.iter().enumerate() {
                            laux::lua_push(state, field);
                            ffi::lua_rawseti(state, -2, (i + 1) as i64);
                        }
                        idx += 1;
                        ffi::lua_rawseti(state, -2, idx as i64);
                    },
                    Err(err) => unsafe {
                        ffi::lua_pushboolean(state, 0);
                        laux::lua_push(
                            state,
                            format!("read csv '{}' error: {}", path.to_string_lossy(), err)
                                .as_str(),
                        );
                        return 2;
                    },
                }
            }

            unsafe {
                ffi::lua_setfield(state, -2, cstr!("data"));
                ffi::lua_rawseti(state, -2, 1);
            }
            1
        }
        Err(err) => {
            unsafe {
                ffi::lua_pushboolean(state, 0);
            }

            laux::lua_push(
                state,
                format!("open file '{}' error: {}", path.to_string_lossy(), err).as_str(),
            );
            2
        }
    }
}

fn read_xlxs(state: LuaState, path: &Path, max_row: usize) -> c_int {
    let res: Result<Xlsx<_>, _> = open_workbook(path);
    match res {
        Ok(mut workbook) => {
            unsafe {
                ffi::lua_createtable(state, 0, 0);
            }
            let mut sheet_counter = 0;
            workbook.sheet_names().iter().for_each(|sheet| {
                if let Ok(range) = workbook.worksheet_range(sheet) {
                    unsafe {
                        ffi::lua_createtable(state, 0, 2);
                        laux::lua_push(state, sheet.as_str());

                        ffi::lua_setfield(state, -2, cstr!("sheet_name"));

                        ffi::lua_createtable(state, range.rows().len() as i32, 0);
                        for (i, row) in range.rows().enumerate() {
                            if i >= max_row {
                                break;
                            }
                            //rows
                            ffi::lua_createtable(state, row.len() as i32, 0);

                            for (j, cell) in row.iter().enumerate() {
                                //columns

                                match cell {
                                    Data::Int(v) => {
                                        ffi::lua_pushinteger(state, *v as ffi::lua_Integer)
                                    }
                                    Data::Float(v) => ffi::lua_pushnumber(state, *v),
                                    Data::String(v) => laux::lua_push(state, v.as_str()),
                                    Data::Bool(v) => ffi::lua_pushboolean(state, *v as i32),
                                    Data::Error(v) => laux::lua_push(state, v.to_string()),
                                    Data::Empty => ffi::lua_pushnil(state),
                                    Data::DateTime(v) => laux::lua_push(state, v.to_string()),
                                    _ => ffi::lua_pushnil(state),
                                }
                                ffi::lua_rawseti(state, -2, (j + 1) as i64);
                            }
                            ffi::lua_rawseti(state, -2, (i + 1) as i64);
                        }
                        ffi::lua_setfield(state, -2, cstr!("data"));
                    }
                    sheet_counter += 1;
                    unsafe {
                        ffi::lua_rawseti(state, -2, sheet_counter as i64);
                    }
                }
            });
            1
        }
        Err(err) => unsafe {
            ffi::lua_pushboolean(state, 0);
            laux::lua_push(state, format!("{}", err).as_str());
            2
        },
    }
}

extern "C-unwind" fn lua_excel_read(state: LuaState) -> c_int {
    let filename: &str = laux::lua_get(state, 1);
    let max_row: usize = laux::lua_opt(state, 2).unwrap_or(usize::MAX);
    let path = Path::new(filename);

    match path.extension() {
        Some(ext) => {
            let ext = ext.to_string_lossy().to_string();
            match ext.as_str() {
                "csv" => read_csv(state, path, max_row),
                "xlsx" => read_xlxs(state, path, max_row),
                _ => unsafe {
                    ffi::lua_pushboolean(state, 0);
                    laux::lua_push(state, format!("unsupport file type: {}", ext));
                    2
                },
            }
        }
        None => unsafe {
            ffi::lua_pushboolean(state, 0);
            laux::lua_push(
                state,
                format!("unsupport file type: {}", path.to_string_lossy()),
            );
            2
        },
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C-unwind" fn luaopen_rust_excel(state: LuaState) -> c_int {
    let l = [lreg!("read", lua_excel_read), lreg_null!()];

    luaL_newlib!(state, l);
    1
}
