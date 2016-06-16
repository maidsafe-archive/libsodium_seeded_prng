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

use std::fmt::{self, Debug, Display, Formatter};
use std::thread;

/// A simple wrapper used to seed the RNG which prints its value on destruction if the current
/// thread is panicking.
#[derive(Clone)]
pub struct Seed([u32; 4]);

impl Seed {
    /// Constructor.
    pub fn new(value: [u32; 4]) -> Seed {
        Seed(value)
    }

    /// Returns the actual value of the seed.
    pub fn value(&self) -> [u32; 4] {
        self.0
    }
}

impl Display for Seed {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "RNG seed: {:?}", self.0)
    }
}

impl Debug for Seed {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        <Self as Display>::fmt(self, formatter)
    }
}

impl Drop for Seed {
    fn drop(&mut self) {
        if thread::panicking() {
            let msg = format!("{}", self);
            let border = (0..msg.len()).map(|_| "=").collect::<String>();
            println!("\n{}\n{}\n{}\n", border, msg, border);
        }
    }
}
