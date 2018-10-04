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

    let size = read_size(ptr);
    println!("size={}", size);

    let result_str = super::deref_str(ptr.offset(8), (size) as usize);
    println!("result string= '{}'", result_str);

    assert_eq!(size, result_str.len() as u64);
    assert_eq!(src_str, src_str);
}}


#[test]
fn integration_sql_test() {

    let create_table = execute_sql("create table USERS(id int, name varchar(128), age int)".to_string());
    assert_eq!(create_table, "table created");
    println!("{}", create_table);

    let insert_one = execute_sql("insert into USERS values(1, 'Sara', 23)".to_string());
    assert_eq!(insert_one, "rows inserted: 1");
    println!("{}", insert_one);

    let insert_several = execute_sql("insert into USERS values(2, 'Bob', 19), (3, 'Caroline', 31), (4, 'Max', 25)".to_string());
    assert_eq!(insert_several, "rows inserted: 3");
    println!("{}", insert_several);

    let empty_select = execute_sql("select * from USERS where name = 'unknown'".to_string());
    assert_eq!(empty_select, "id, name, age\n");
    println!("{}", empty_select);

    let select_all = execute_sql("select id, name from USERS".to_string());
    println!("{}", select_all);
    assert_eq!(select_all, "id, name\n1, Sara\n2, Bob\n3, Caroline\n4, Max");

    let explain = execute_sql("explain select id, name from USERS".to_string());
    println!("{}", explain);
    assert_eq!(explain, "query plan\n".to_string() +
                        "column names: (`id`, `name`)\n" +
                        "(scan `users` :source-id 0\n" +
                        "  (yield\n" +
                        "    (column-field :source-id 0 :column-offset 0)\n" +
                        "    (column-field :source-id 0 :column-offset 1)))"
    );



}

//
// Private helper functions.
//

/// Execute sql and returns result as String.
fn execute_sql(sql: String) -> String { unsafe {

    // converts params
    let sql_str_ptr = super::put_to_mem(sql);
    let sql_str_len = read_size(sql_str_ptr) as usize;

    // executes query
    let result_str_ptr = super::do_query(sql_str_ptr.offset(8), sql_str_len) as *mut u8;
    // converts results
    let result_str_len = read_size(result_str_ptr) as usize;
    let result_str = super::deref_str(result_str_ptr.offset(8), result_str_len);

    result_str
}}

/// Reads u64 from current pointer.
unsafe fn read_size(ptr: *mut u8) -> u64 {
    let mut size_as_bytes: [u8; 8] = [0; 8];
    for idx in 0..8 {
        let byte = std::ptr::read(ptr.offset(idx));
        size_as_bytes[7 - idx as usize] = byte;
    }

    std::mem::transmute(size_as_bytes)
}
