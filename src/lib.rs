#[rustfmt::skip]
#[macro_use]
pub mod macros;

pub mod collections;
pub mod eval;
pub mod gen;
pub mod load;
pub mod max_lines;

type HB = std::hash::BuildHasherDefault<fnv::FnvHasher>;

mod fnv {
    use std::hash::Hasher;
    pub struct FnvHasher(u64);

    impl Default for FnvHasher {
        #[inline]
        fn default() -> FnvHasher {
            FnvHasher(0xcbf29ce484222325)
        }
    }

    impl Hasher for FnvHasher {
        #[inline]
        fn finish(&self) -> u64 {
            self.0
        }

        #[inline]
        fn write(&mut self, bytes: &[u8]) {
            let FnvHasher(mut hash) = *self;

            for byte in bytes.iter() {
                hash = hash ^ (*byte as u64);
                hash = hash.wrapping_mul(0x100000001b3);
            }

            *self = FnvHasher(hash);
        }
    }
}
