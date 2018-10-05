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
    println!("{}", create_table);
    assert_eq!(create_table, "table created");

    let insert_one = execute_sql("insert into USERS values(1, 'Sara', 23)".to_string());
    println!("{}", insert_one);
    assert_eq!(insert_one, "rows inserted: 1");

    let insert_several = execute_sql("insert into USERS values(2, 'Bob', 19), (3, 'Caroline', 31), (4, 'Max', 25)".to_string());
    println!("{}", insert_several);
    assert_eq!(insert_several, "rows inserted: 3");

    let empty_select = execute_sql("select * from USERS where name = 'unknown'".to_string());
    println!("{}", empty_select);
    assert_eq!(empty_select, "id, name, age\n");

    let select_all = execute_sql("select id, name from USERS".to_string());
    assert_eq!(select_all, "id, name\n1, Sara\n2, Bob\n3, Caroline\n4, Max");
    println!("{}", select_all);

    let create_table_role = execute_sql("create table ROLES(user_id int, role varchar(128))".to_string());
    println!("{}", create_table_role);
    assert_eq!(create_table_role, "table created");

    let insert_roles = execute_sql("insert into ROLES values(1, 'Teacher'), (2, 'Student'), (3, 'Scientist'), (4, 'Writer')".to_string());
    println!("{}", insert_roles);
    assert_eq!(insert_roles, "rows inserted: 4");

    let select_with_join = execute_sql("select u.name as Name, r.role as Role from USERS u join ROLES r on u.id = r.user_id where r.role = 'Writer'".to_string());
    println!("{}", select_with_join);
    assert_eq!(select_with_join, "name, role\nMax, Writer");


    let explain = execute_sql("explain select id, name from USERS".to_string());
    println!("{}", explain);
    assert_eq!(explain, "query plan\n".to_string() +
                        "column names: (`id`, `name`)\n" +
                        "(scan `users` :source-id 0\n" +
                        "  (yield\n" +
                        "    (column-field :source-id 0 :column-offset 0)\n" +
                        "    (column-field :source-id 0 :column-offset 1)))"
    );

    let bad_query = execute_sql("select salary from USERS".to_string());
    println!("{}", bad_query);
    assert_eq!(bad_query, "[Error] column does not exist: salary");

    // delete and update is not supported by llamadb

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
