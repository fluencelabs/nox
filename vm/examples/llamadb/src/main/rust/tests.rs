//! Module with tests.


#[test]
fn alloc_dealloc_test() {
    unsafe {
        let size = 100;
        let ptr = super::allocate(size);
        assert_eq!(super::deallocate(ptr, size), ());
    }
}

#[test]
fn write_str_to_mem_test() { unsafe {
    let src_str = "some string Ω".to_string();
    let ptr = super::put_to_mem(src_str.clone());

    let mut size_as_bytes: [u8; 8] = [0; 8];
    for idx in 0..8 {
        let byte = std::ptr::read(ptr.offset(idx));
        size_as_bytes[7 - idx as usize] = byte;
    }

    let size: u64 = std::mem::transmute(size_as_bytes);
    println!("size={}", size);

    let result_str = super::deref_str(ptr.offset(size_as_bytes.len() as isize), (size) as usize);
    println!("result string= '{}'", result_str);

    assert_eq!(size, result_str.len() as u64);
    assert_eq!(src_str, src_str);
}}