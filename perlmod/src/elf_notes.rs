//! The `#[package]` macro stores package lists in ELF file note sections.
//! This is the implementation for this.
//!
//! This is private and not meant to be a public API.

/*
#[repr(C, packed)]
pub struct Info {
    extra stuff
}
*/

#[repr(C, align(4))]
pub struct ElfNote<const N: usize> {
    pub name_size: u32,
    pub desc_size: u32,
    pub ty: u32,
    pub name: [u8; N],
    //pub desc: Info,
}

impl<const N: usize> ElfNote<{ N }> {
    pub const fn new_package(name: [u8; N]) -> Self {
        Self {
            name_size: N as u32,
            desc_size: 0, // size_of::<Info>()
            ty: 0,
            name,
            //desc: Info::new(),
        }
    }
}
