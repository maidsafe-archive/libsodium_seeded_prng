// Copyright 2016 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under (1) the MaidSafe.net Commercial License,
// version 1.0 or later, or (2) The General Public License (GPL), version 3, depending on which
// licence you accepted on initial access to the Software (the "Licences").
//
// By contributing code to the SAFE Network Software, or to this project generally, you agree to be
// bound by the terms of the MaidSafe Contributor Agreement, version 1.0.  This, along with the
// Licenses can be found in the root directory of this project at LICENSE, COPYING and CONTRIBUTOR.
//
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.
//
// Please review the Licences for the specific language governing permissions and limitations
// relating to use of the SAFE Network Software.

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::iter::repeat;
use std::rc::Rc;
use std::str;
use std::sync::Mutex;

use error::Error;
use rand::{self, SeedableRng, XorShiftRng};
use xor_shift_seed::Seed;

lazy_static! {
    static ref INIT_RESULT: Mutex<Option<Result<Seed, Error>>> = Mutex::new(None);
    static ref SEEDED_RANDOM: Mutex<SeededRandom> = Mutex::new(SeededRandom::default());
}

thread_local!(static RNG: Rc<RefCell<XorShiftRng>> =
    Rc::new(RefCell::new(XorShiftRng::from_seed(get_seed().value()))));

struct SeededRandom {
    function_pointers: ffi::FunctionPointers,
    name: CString,
    seed: [u32; 4],
}

impl Default for SeededRandom {
    fn default() -> SeededRandom {
        let seed = [rand::random(), rand::random(), rand::random(), rand::random()];
        SeededRandom {
            function_pointers: ffi::FunctionPointers::default(),
            name: unwrap!(CString::new("Rust XorShiftRng")),
            seed: seed,
        }
    }
}

mod ffi {
    use libc::{c_char, c_int, c_void, size_t, uint32_t};
    use rand::Rng;

    #[repr(C)]
    pub struct FunctionPointers {
        implementation_name: extern "C" fn() -> *const c_char,
        random: extern "C" fn() -> uint32_t,
        stir: Option<extern "C" fn()>,
        uniform: Option<extern "C" fn(upper_bound: uint32_t) -> uint32_t>,
        buf: extern "C" fn(buf: *mut c_void, size: size_t),
        close: Option<extern "C" fn() -> c_int>,
    }

    impl Default for FunctionPointers {
        fn default() -> FunctionPointers {
            FunctionPointers {
                implementation_name: implementation_name,
                random: random,
                stir: None,
                uniform: None,
                buf: buf,
                close: None,
            }
        }
    }

    #[link(name="sodium")]
    extern "C" {
        pub fn randombytes_set_implementation(function_pointers: *mut FunctionPointers) -> c_int;
        pub fn randombytes_implementation_name() -> *const c_char;
        pub fn randombytes_random() -> uint32_t;
        pub fn randombytes_uniform(upper_bound: uint32_t) -> uint32_t;
        pub fn randombytes_buf(buf: *mut u8, size: size_t);
        pub fn sodium_init() -> c_int;
    }

    extern "C" fn implementation_name() -> *const c_char {
        unwrap!(super::SEEDED_RANDOM.lock()).name.as_ptr()
    }

    extern "C" fn random() -> uint32_t {
        super::RNG.with(|rng| rng.borrow_mut().gen())
    }

    #[cfg_attr(feature="clippy", allow(cast_possible_wrap))]
    #[allow(unsafe_code)]
    extern "C" fn buf(buf: *mut c_void, size: size_t) {
        unsafe {
            let ptr = buf as *mut u8;
            let rng_ptr = super::RNG.with(|rng| rng.clone());
            let rng = &mut *rng_ptr.borrow_mut();
            for i in 0..size {
                *ptr.offset(i as isize) = rng.gen();
            }
        }
    }
}

/// Returns the name of this libsodium `randombytes` implementation.
#[allow(unsafe_code)]
pub fn implementation_name() -> String {
    let name_string = unsafe { CStr::from_ptr(ffi::randombytes_implementation_name()).to_bytes() };
    unwrap!(str::from_utf8(name_string)).to_owned()
}

/// Returns a random `u32`.
#[allow(unsafe_code)]
pub fn random_u32() -> u32 {
    unsafe { ffi::randombytes_random() }
}

/// Returns a random `u32` between 0 and `upper_bound` (excluded).
///
/// Unlike [`random_u32()`](fn.random_u32.html)` % upper_bound`, it does its best to guarantee a
/// uniform distribution of the possible output values.
#[allow(unsafe_code)]
pub fn random_u32_uniform(upper_bound: u32) -> u32 {
    unsafe { ffi::randombytes_uniform(upper_bound) }
}

/// Returns a vector of random bytes of length `size`.
#[allow(unsafe_code)]
pub fn random_bytes(size: usize) -> Vec<u8> {
    unsafe {
        let mut buf: Vec<u8> = repeat(0u8).take(size).collect();
        ffi::randombytes_buf(buf.as_mut_ptr(), size);
        buf
    }
}

/// Returns a copy of the current RNG seed.
pub fn get_seed() -> Seed {
    Seed::new(unwrap!(SEEDED_RANDOM.lock()).seed)
}

/// Sets libsodium `randombytes` to this implementation and initialises libsodium.
///
/// If `optional_seed` is `Some`, then the RNG is seeded with this value, unless `init()` has
/// previously been called and the current seed is different to the requested one, in which case
/// `Err(Error::AlreadySeeded)` is returned.
///
/// This function is safe to call multiple times concurrently from different threads.
#[allow(unsafe_code)]
pub fn init(optional_seed: Option<[u32; 4]>) -> Result<Seed, Error> {
    let mut init_result = &mut *unwrap!(INIT_RESULT.lock());
    if let Some(ref existing_result) = *init_result {
        // Return error if seed passed in here is different to current one.
        if let Ok(ref existing_seed) = *existing_result {
            if let Some(ref new_seed) = optional_seed {
                if *new_seed != existing_seed.value() {
                    return Err(Error::AlreadySeeded);
                }
            }
        }
        return (*existing_result).clone();
    }
    let mut sodium_result;
    {
        let seeded_random = &mut *unwrap!(SEEDED_RANDOM.lock());
        if let Some(value) = optional_seed {
            seeded_random.seed = value;
        }
        sodium_result =
            unsafe { ffi::randombytes_set_implementation(&mut seeded_random.function_pointers) };
    }
    match sodium_result {
        // Note that this function only ever calls libsodium's `init()` once.  This is reasonable
        // given the current implementation of libsodium's `init()`.  However if that should change
        // to make it worth retrying after a failed `init()`, then this function should be updated
        // too.
        0 => sodium_result = unsafe { ffi::sodium_init() },
        _ => (),
    };
    let overall_result = match sodium_result {
        0 => Ok(get_seed()),
        result => Err(Error::Libsodium(result)),
    };
    *init_result = Some(overall_result.clone());
    overall_result
}

/// Return a copy of the thread-local RNG pointer
pub fn get_rng() -> Rc<RefCell<XorShiftRng>> {
    RNG.with(|rng| rng.clone())
}



#[cfg(test)]
mod tests {
    use super::*;
    use error::Error;
    use sodiumoxide::crypto::box_;

    const SEED_VALUE: [u32; 4] = [0, 1, 2, 3];

    #[test]
    fn seeded() {
        let seed = unwrap!(init(Some(SEED_VALUE)));
        assert_eq!(seed.value(), SEED_VALUE);
        assert_eq!(get_seed().value(), SEED_VALUE);

        // Initialise with same seed again - should succeed.
        assert_eq!(unwrap!(init(Some(SEED_VALUE))).value(), SEED_VALUE);

        // Initialise with no seed - should succeed.
        assert_eq!(unwrap!(init(None)).value(), SEED_VALUE);

        // Initialise with different seed - should fail.
        if let Err(Error::AlreadySeeded) = init(Some([0, 0, 0, 0])) {} else {
            panic!("Unexpected result")
        }

        let mut random_u32s = vec![];
        for _ in 0..3 {
            random_u32s.push(random_u32());
            random_u32s.push(random_u32_uniform(100));
        }
        assert_eq!(random_u32s, [809904348, 34, 331598031, 92, 29475044, 66]);

        assert_eq!(random_bytes(10), [189, 36, 9, 209, 239, 95, 69, 207, 163, 2]);

        let (public_key, private_key) = box_::gen_keypair();
        let expected_public_key = [40, 10, 48, 161, 184, 192, 94, 70, 25, 185, 154, 217, 37, 186,
                                   12, 113, 148, 176, 1, 7, 189, 118, 184, 249, 160, 220, 159, 78,
                                   111, 46, 223, 20];
        assert_eq!(expected_public_key, public_key.0);
        let expected_private_key = [37, 237, 255, 64, 206, 191, 101, 123, 66, 38, 178, 123, 129,
                                    245, 169, 102, 250, 68, 136, 38, 172, 196, 64, 161, 177, 248,
                                    224, 146, 98, 147, 140, 46];
        assert_eq!(expected_private_key, private_key.0);

        assert_eq!("Rust XorShiftRng".to_owned(), implementation_name());
    }
}
