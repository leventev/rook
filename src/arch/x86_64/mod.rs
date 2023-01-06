pub mod paging;

extern "C" {
    #[link_name = "x86_64_get_cr3"]
    pub fn get_cr3() -> u64;

    #[link_name = "x86_64_set_cr3"]
    pub fn set_cr3(cr3: u64);

    #[link_name = "x86_64_get_rflags"]
    pub fn get_rflags() -> u64;
}
