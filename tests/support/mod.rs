// tests/support/mod.rs
// The helpers and mocks modules are test-only support code used by multiple
// integration test binaries. Some symbols are purposely unused in individual
// test crates which causes dead_code / unused_imports warnings. Allow those
// warnings at the module level to keep CI output clean.
#[allow(dead_code, unused_imports)]
pub mod mocks;

#[allow(dead_code, unused_imports)]
pub mod helpers;

#[allow(dead_code, unused_imports)]
pub mod builders;

#[allow(unused_imports)]
pub use mocks::*;

#[allow(unused_imports)]
pub use helpers::*;
#[allow(unused_imports)]
pub use builders::*;
// This is a clean module file without duplicates.
