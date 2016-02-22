extern crate libc;
use std::slice;
use std::ffi::CStr;

use libc::c_int;
use libc::c_void;
use libc::c_char;
use libc::c_uint;

#[link(name = "varnishapi")]
extern {

	static VSL_tags: *const c_char;
	pub fn VSM_New() -> *const c_void;
	pub fn VSM_Delete(vd: *const c_void);
	pub fn VSM_Error(vd: *const c_void) -> * const c_char;
	pub fn VSM_ResetError(vd: *const c_void);
	pub fn VSM_Name(vd: *const c_void) -> * const c_char;
	pub fn VSM_Open(vd: *const c_void) -> c_int;
	pub fn VSM_IsOpen(vd: *const c_void) -> c_int;
	pub fn VSM_Abandoned(vd: *const c_void) -> c_int;
	pub fn VSM_Close(vd: *const c_void);
	pub fn VSC_Arg(vd: *const c_void, arg: c_int, opt: *const c_char ) -> c_int;
	pub fn VSC_Iter(vd: *const c_void,
			fantom: *const c_void,
			cb_bounce: extern fn (*mut c_void, *const VSC_point) -> c_int,
			cb: *const c_void);
	pub fn VSL_New() -> *const c_void;
	pub fn VSL_Next(c: *const VSL_cursor) -> c_int;
	pub fn VSLQ_SetCursor(vslq: *const c_void, cur: *const *const c_void);
	pub fn VSL_CursorVSM(vsl: *const c_void, vsm: *const c_void, opt: c_uint) -> *const c_void;
	pub fn VSLQ_New(vsl:*const c_void, c: *const c_void, grouping: c_int, meh: *const c_void) -> *const c_void;
	pub fn VSLQ_Delete(vslq: *const *const c_void);
	pub fn VSLQ_Dispatch(vslq: *const c_void,
			     cb_bounce : extern fn (_: *const c_void, 
						    pt: *const *const VslTransaction,
						    cb: *const c_void) -> c_int,
			     cb: *const c_void
			    ) -> c_int;
	pub fn VSL_DeleteCursor(cur:*const c_void);
	pub fn VSL_Delete(vsl:*const c_void);
}

macro_rules! conv {
	( $( $x:expr ),* ) => {
		{
			$(
				CStr::from_ptr($x as *const i8)
				.to_str()
				.unwrap()
			 )*
		}
	};
}

enum VsmChunk {}

#[repr(C)]
pub struct VSM_fantom {
	chunk: *const VsmChunk,
	b: *const c_char,
	e: *const c_char,
	prv: *const c_void,
	class: [c_char; 8],
	typ: [c_char; 8],
	ident: [c_char; 128],
}

#[repr(C)]
pub struct VSC_level_desc {
	verbosity: *const c_int,
	label: *const c_char,
	sdesc: *const c_char,
	ldesc: *const c_char,
}

#[repr(C)]
pub struct VSC_type_desc {
	label: *const c_char,
	sdesc: *const c_char,
	ldesc: *const c_char,
}

#[repr(C)]
pub struct VSC_section {
	typ: *const c_char,
	ident: *const c_char,
	desc: *const VSC_type_desc,
	fantom: *const VSM_fantom
}

#[repr(C)]
pub struct VSC_desc {
	name: *const c_char,
	ctype: *const c_char,
	semantics: c_int,
	format: c_int,
	level: *const VSC_level_desc,
	sdesc: *const c_char,
	ldesc: *const c_char,
}

#[repr(C)]
pub struct VSC_point {
	desc: *const VSC_desc,
	ptr: *const c_void,
	section: *const VSC_section
}

#[repr(C)]
pub struct VSL_cursor {
	ptr: *const u32,
}

pub enum VslReason {
	Unknown,
	Http1,
	RxReq,
	Esi,
	Restart,
	Pass,
	Fetch,
	BgFetch,
	Pipe
}

pub enum VslType {
	Unknown,
	Sess,
	Req,
	BeReq,
	Raw,
}

#[repr(C)]
pub struct VslTransaction<'a> {
	pub level: c_uint,
	pub vxid: i32,
	pub vxid_parent: i32,
	pub typ: c_uint,
	pub reason: c_uint,
	c: &'a VSL_cursor
}

impl<'a> VSL_cursor {
	pub fn get_string(&self) -> &'a str {
		unsafe { conv!(self.ptr.offset(2) as *const i8) }
	}

	pub fn get_ntag(&self) -> u8 {
		unsafe { (*(self.ptr) >> 24) as u8 }
	}

	pub fn get_stag(&self) -> &str {
		let p: *const *const c_char= &VSL_tags;
		unsafe {
			conv!(*p.offset(self.get_ntag() as isize))
		}
	}
}

impl<'a> Iterator for &'a VslTransaction<'a> {
	type Item = &'a VSL_cursor;
	fn next(&mut self) -> Option<&'a VSL_cursor> {
		match unsafe { VSL_Next(self.c) } {
			0 => None,
			_ => Some(self.c as &VSL_cursor)
		}
	}
}

pub struct VscSection<'a> {
	//pub typ: &'a str ,
	pub ident: &'a str,
	//pub label: &'a str, 
	pub sdesc: &'a str,
	pub ldesc: &'a str,
}

pub struct VscDesc<'a> {
	pub name: &'a str,
	pub semantics: char,
	pub format: char,
	//level
	pub sdesc: &'a str,
	pub ldesc: &'a str,
}

pub struct VsmEntry<'a> {
	pub t: &'a str,
	pub ident: &'a str,
	pub value: u64,
	pub desc: VscDesc<'a>,
	pub section: VscSection<'a>
}

pub extern "C" fn log_bounce<F>(_: *const c_void, 
				 pt: *const *const VslTransaction, 
				 cb: *const c_void) -> c_int
		where F: FnMut(& [&VslTransaction]) -> bool {
	let cb = cb as *mut F;
	let mut n = 0;
	let s;

	unsafe {
		loop {
			if (*pt.offset(n)).is_null()  {
				break;
			}
			n += 1;
		}
		s = slice::from_raw_parts::<&VslTransaction>(pt as *const &VslTransaction, n as usize) ;
	}

	match unsafe { (*cb)(s) } {
			true => 1,
			false =>1
	}
}

pub extern "C" fn stat_bounce<F>(cb: *mut c_void,
			pt: *const VSC_point) -> c_int
		where F: FnMut(&VsmEntry) -> bool {
	if pt.is_null() {
		return 0;
	}
	let entry;
	unsafe {
		assert!(!(*pt).section.is_null());
		let fantom = (*(*pt).section).fantom;
		assert!(!fantom.is_null());
		assert!(conv!(&(*fantom).class) == "Stat");

		let desc = (*pt).desc;
		assert!(!desc.is_null());
		assert!(conv!((*desc).ctype) == "uint64_t");

		let section = (*pt).section;
		assert!(!section.is_null());
		assert!(conv!((*section).typ) == conv!(&(*fantom).typ));
		assert!(conv!((*section).ident) == conv!(&(*fantom).ident));

		entry = VsmEntry {
			t: conv!(&(*fantom).typ),
			ident: conv!(&(*fantom).ident),
			value: *((*pt).ptr as *const u64),
			desc: VscDesc {
				name: conv!((*desc).name),
				semantics: ((*desc).semantics as u8) as char,
				format: ((*desc).format as u8) as char,
				sdesc: conv!((*desc).sdesc),
				ldesc: conv!((*desc).ldesc)
			},
			section: VscSection {
				ident: conv!((*section).ident),
				sdesc: conv!((*(*section).desc).sdesc),
				ldesc: conv!((*(*section).desc).ldesc)
			}
		};
	}
	let cb = cb as *mut F;
	let r;
	unsafe {
		r = (*cb)(&entry);
	}
	match r {
		true => 0,
		false =>1
	}
}
