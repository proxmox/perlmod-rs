//! Module for digests to test string/byte parameters.

/// FNV64a should be good enough for simple string tests, we don't need cryptographic strength, and
/// we can get this in a few lines without any dependencies.
fn fnv64a<T: ?Sized + AsRef<[u8]>>(buf: &T) -> u64 {
    buf.as_ref()
        .into_iter()
        .fold(0xcbf29ce484222325, |hval, &b| {
            (hval ^ u64::from(b)).wrapping_mul(0x100000001b3)
        })
}

#[test]
fn test_hashes() {
    const HELLO_YOU_H: u64 = 0xdb61ca777f4b8ba0;
    let hash = fnv64a("Hello You");
    assert_eq!(hash, HELLO_YOU_H, "hash = 0x{hash:x}, expected 0x{LO_H:x}");

    const LO_H: u64 = 0x1250b4191dafc2a4;
    let hash = fnv64a("lo ");
    assert_eq!(hash, LO_H, "hash = 0x{hash:x}, expected 0x{LO_H:x}");

    const EMO_ROBOT_H: u64 = 0x5631099f8622bde8;
    let hash = fnv64a("emoji 🤖");
    assert_eq!(
        hash, EMO_ROBOT_H,
        "hash = 0x{hash:x}, expected 0x{EMO_ROBOT_H:x}"
    );
}

#[perlmod::package(name = "TestLib::Digest", lib = "testlib")]
mod export {
    #[export]
    pub fn fnv64a(s: &[u8]) -> u64 {
        super::fnv64a(s)
    }
}
