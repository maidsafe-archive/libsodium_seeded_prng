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

quick_error! {
    /// Error types
    #[derive(Clone, Debug)]
    pub enum Error {
        /// libsodium error
        Libsodium(error: i32) {
            description("libsodium error")
            display("libsodium error: {}", error)
            from()
        }
        /// Returned by [`init()`](fn.init.html) if it is called with a seed, but where `init()`
        /// has already been called and is using a different seed.
        AlreadySeeded {
            description("already seeded")
            display("RNG has already been initialised with a different seed.")
        }
    }
}
