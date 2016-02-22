extern crate libc;
use std::ptr;
use std::ffi::CStr;
use std::ffi::CString;
use std::thread::sleep;
use std::time::Duration;

use libc::c_int;
use libc::c_void;

mod ffi;
pub use self::ffi::VslTransaction;
pub use self::ffi::VslReason;
pub use self::ffi::VslType;


/// VsmType allows you to specify the location of the shared memory, as well
/// as its type
pub enum VsmType<'a> {
	/// A Varnish instance is running with this name
	Active(&'a str),
	/// Use an abandonned vsm file
	Stale(&'a str),
	/// Use the default vsm location
	Default
}

/// Describe the different types of data you can retrieve using `VsmData::stats`
pub enum Semantics {
	/// Bitmap value
	Bitmap,
	/// Counter, never decreases
	Counter,
	/// Gauge, goes up and down
	Gauge,
	/// Couldn't make out the type, but, well, here you go
	Unknown
}

/// One item retrieved from the VSM
pub struct Stats {
	/// Name constructed using the type, possibly ident of the C entry,
	/// suffixed by the C description name
	pub name: String,
	/// Value of that item
	pub value: u64,
	/// Type of that item
	pub semantics: Semantics
}

/// VSMData objects allow you to connect to the Varnish Shared Memory space
/// and iterate of all exposed counters
pub struct VsmData {
	vsm: *const c_void,
}


impl VsmData {

	/// Creates a new VSmData object and maybe points it to a specific
	/// vsm file.
	pub fn new(t: VsmType) -> Result<VsmData, &str> {
		let p;
		unsafe {
			p = ffi::VSM_New();
		}
		if p.is_null() {
			return Err("VSL_New return an empty pointer");
		}
		match t {
			VsmType::Active(s) => unsafe {
				assert!(ffi::VSC_Arg(p, 'n' as c_int,
						CString::new(s).unwrap().
							as_ptr()) > 0);
			},
			VsmType::Stale(s) => unsafe {
				assert!(ffi::VSC_Arg(p, 'N' as c_int,
						CString::new(s).unwrap().
							as_ptr()) > 0);
			},
			VsmType::Default => {},
		}
		let mut vd = VsmData { vsm: p };
		if vd.open() == 0 {
			Ok(vd)
		} else {
			let message = vd.error();
			vd.reset_error();
			Err(message)
		}

	}

	/// Actually connects the object to the log. It may fail for various
	/// reasons, but you'll have to look at the error string to know which
	/// one.
	fn open(&mut self) -> c_int {
		assert!(!self.vsm.is_null());
		unsafe {
			ffi::VSM_Open(self.vsm)
		}
	}

	fn error(&self) -> &'static str {
		assert!(!self.vsm.is_null());
		let s;
		unsafe {
			s = ffi::VSM_Error(self.vsm);
		}

		if s.is_null() {
			"No error"
		} else {
			unsafe {
				CStr::from_ptr(s).to_str().unwrap()
				//CStr::from_ptr(s).to_string_lossy().into_owned()
			}
		}
	}

	fn reset_error(&mut self) {
		assert!(!self.vsm.is_null());
		unsafe { ffi::VSM_ResetError(self.vsm); };
	}

	fn stat_iter<F>(&self, cb: F) where F: FnMut(&ffi::VsmEntry) -> bool {
		assert!(!self.vsm.is_null());
		let cb = &cb as *const _ as *const c_void;
		unsafe {
			ffi::VSC_Iter(self.vsm,
				 ptr::null(),
				 ffi::stat_bounce::<F>,
				 &cb as *const _ as *mut c_void);
		}
	}

	/// returns a vector of owned `Stats`
	pub fn stats(&self) -> Vec<Stats> {
		assert!(!self.vsm.is_null());
		let mut v = Vec::new();
		self.stat_iter(
			|e: &ffi::VsmEntry| -> bool {
				let mut name = e.t.to_string();
				if !e.ident.is_empty() {
					name = name + "." + e.ident;
				}
				name = name + "." + e.desc.name;
				println!("{}: {}", name, e.value);
				v.push(Stats {
					name: name,
					value: e.value,
					semantics: match e.desc.semantics {
						'b' => Semantics::Bitmap,
						'c' => Semantics::Counter,
						'g' => Semantics::Gauge,
						_ => Semantics::Unknown,
					}
				});
				true
			}
			
			);
		v
	}
	/// Reads the log 
	pub fn log_iter<F>(&self, cb: F)
		where F: FnMut(& [&VslTransaction]) -> bool {
		// TODO: replace assert with errors
		let cb = &cb as *const _ as *const c_void;
		let vsl = unsafe { ffi::VSL_New() }; 
		assert!(!vsl.is_null());
		let cur = unsafe { ffi::VSL_CursorVSM(vsl, self.vsm, 3) };
		assert!(!cur.is_null());
		let vslq = unsafe { ffi::VSLQ_New(vsl, ptr::null(), 1, ptr::null()) };
		assert!(!vslq.is_null());
		unsafe { ffi::VSLQ_SetCursor(vslq, &cur) };
		loop {
			match unsafe {ffi::VSLQ_Dispatch(vslq,
					    ffi::log_bounce::<F>,
					    &cb as *const _ as *mut c_void)} {
				1 => { continue; }
				0 => {
					sleep(Duration::from_millis(10));
					continue;
				}
				_ => { break; }
			}
		}
		unsafe { ffi::VSLQ_Delete(&vslq) };
		unsafe { ffi::VSL_DeleteCursor(vslq) };
		unsafe { ffi::VSL_Delete(vsl) };
	}

	pub fn log(&self) {
		self.log_iter( |pt| -> bool {
			for t in pt {
				println!("=> vxid: {}", t.vxid);
				println!("=> vxid_parent: {}", t.vxid_parent);
				println!("=> type: {}", t.typ);
				println!("=> reason: {}", t.reason);
				for c in *t {
					println!("{:8}\t{:8}\t{}", c.get_stag(), c.get_ntag(), c.get_string());
				}
			}
			true
		}
		);
		println!("out log");
	}

	/// Return the file location being used.
	pub fn name(&self) -> String {
		unsafe {
			let s = ffi::VSM_Name(self.vsm);
			CStr::from_ptr(s).to_string_lossy().into_owned()
		}
	}

	/// Check if the VsmData object is open
	pub fn is_open(&self) -> bool {
		assert!(!self.vsm.is_null());
		unsafe { ffi::VSM_IsOpen(self.vsm).is_positive() }
	}

	/// Check if the Varnish instance dropped the VSM
	pub fn is_abandoned(&self) -> bool {
		assert!(!self.vsm.is_null());
		unsafe { ffi::VSM_Abandoned(self.vsm).is_positive() }
	}

	/// Close the VSM connection
	pub fn close(&mut self) {
		assert!(!self.vsm.is_null());
		unsafe { ffi::VSM_Close(self.vsm); };
	}
}

impl Drop for VsmData {
	fn drop(&mut self) {
		unsafe { ffi::VSM_Delete(self.vsm);}
	}
}
