//! Module with tests.

use STR_LEN_BYTES;

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

    let result_str = super::deref_str(ptr.offset(STR_LEN_BYTES as isize), (size) as usize);
    println!("result string= '{}'", result_str);

    assert_eq!(size, result_str.len() as u32);
    assert_eq!(src_str, src_str);
}}


#[test]
fn integration_sql_test() {

    //
    // Success cases.
    //

    let create_table = execute_sql("create table Users(id int, name varchar(128), age int)".to_string());
    println!("{}", create_table);
    assert_eq!(create_table, "table created");

    let insert_one = execute_sql("insert into Users values(1, 'Sara', 23)".to_string());
    println!("{}", insert_one);
    assert_eq!(insert_one, "rows inserted: 1");

    let insert_several = execute_sql("insert into Users values(2, 'Bob', 19), (3, 'Caroline', 31), (4, 'Max', 25)".to_string());
    println!("{}", insert_several);
    assert_eq!(insert_several, "rows inserted: 3");

    let create_table_role = execute_sql("create table Roles(user_id int, role varchar(128))".to_string());
    println!("{}", create_table_role);
    assert_eq!(create_table_role, "table created");

    let insert_roles = execute_sql("insert into Roles values(1, 'Teacher'), (2, 'Student'), (3, 'Scientist'), (4, 'Writer')".to_string());
    println!("{}", insert_roles);
    assert_eq!(insert_roles, "rows inserted: 4");

    let empty_select = execute_sql("select * from Users where name = 'unknown'".to_string());
    println!("{}", empty_select);
    assert_eq!(empty_select, "id, name, age\n");

    let select_all = execute_sql("select * from Roles".to_string());
    assert_eq!(select_all, "user_id, role\n1, Teacher\n2, Student\n3, Scientist\n4, Writer");
    println!("{}", select_all);

    let select_with_join = execute_sql("select u.name as Name, r.role as Role from Users u join Roles \
    r on u.id = r.user_id where r.role = 'Writer'".to_string());
    println!("{}", select_with_join);
    assert_eq!(select_with_join, "name, role\nMax, Writer");

    let explain = execute_sql("explain select id, name from Users".to_string());
    println!("{}", explain);
    assert_eq!(explain, "query plan\n".to_string() +
                        "column names: (`id`, `name`)\n" +
                        "(scan `users` :source-id 0\n" +
                        "  (yield\n" +
                        "    (column-field :source-id 0 :column-offset 0)\n" +
                        "    (column-field :source-id 0 :column-offset 1)))"
    );

    let delete = execute_sql("delete from Users where id = (select user_id from Roles where role = 'Student');".to_string());
    println!("{}", delete);
    assert_eq!(delete, "rows deleted: 1");

    //
    // Error cases.
    //

    let empty_str = execute_sql("".to_string());
    println!("{}", empty_str);
    assert_eq!(empty_str, "[Error] Expected SELECT, INSERT, CREATE, or EXPLAIN statement; got no more tokens");

    let invalid_sql = execute_sql("123".to_string());
    println!("{}", invalid_sql);
    assert_eq!(invalid_sql, "[Error] Expected SELECT, INSERT, CREATE, or EXPLAIN statement; got Number(\"123\")");

    let bad_query = execute_sql("select salary from Users".to_string());
    println!("{}", bad_query);
    assert_eq!(bad_query, "[Error] column does not exist: salary");

    let lexer_error = execute_sql("π".to_string());
    println!("{}", lexer_error);
    assert_eq!(lexer_error, "[Error] Lexer error: Unknown character π");

    let incompatible_types = execute_sql("select * from Users where age = 'Bob'".to_string());
    println!("{}", incompatible_types);
    assert_eq!(incompatible_types, "[Error] 'Bob' cannot be cast to Integer { signed: true, bytes: 8 }");

    // Not supported operations

    let not_supported_truncate = execute_sql("truncate table Users".to_string());
    println!("{}", not_supported_truncate);
    assert_eq!(not_supported_truncate, "[Error] Expected SELECT, INSERT, CREATE, or EXPLAIN statement; got Ident(\"truncate\")");

    let not_supported_drop = execute_sql("drop table Users".to_string());
    println!("{}", not_supported_drop);
    assert_eq!(not_supported_drop, "[Error] Expected SELECT, INSERT, CREATE, or EXPLAIN statement; got Ident(\"drop\")");

    let not_supported_update= execute_sql("update Users set name = 'Rob' where name = 'Bob'".to_string());
    println!("{}", not_supported_update);
    assert_eq!(not_supported_update, "[Error] Expected SELECT, INSERT, CREATE, or EXPLAIN statement; got Update");

    let not_supported_order_by = execute_sql("select * from Users order by name".to_string());
    println!("{}", not_supported_order_by);
    assert_eq!(not_supported_order_by, "[Error] order by in not implemented");

}

//
// Private helper functions.
//

/// Executes sql and returns result as a String.
fn execute_sql(sql: String) -> String { unsafe {

    // converts params
    let sql_str_ptr = super::put_to_mem(sql);
    let sql_str_len = read_size(sql_str_ptr) as usize;

    // executes query
    let result_str_ptr = super::do_query(sql_str_ptr.offset(STR_LEN_BYTES as isize), sql_str_len) as *mut u8;
    // converts results
    let result_str_len = read_size(result_str_ptr) as usize;
    let result_str = super::deref_str(result_str_ptr.offset(STR_LEN_BYTES as isize), result_str_len);

    result_str
}}

/// Reads u32 from current pointer.
unsafe fn read_size(ptr: *mut u8) -> u32 {
    let mut size_as_bytes: [u8; STR_LEN_BYTES] = [0; STR_LEN_BYTES];
    for idx in 0..(STR_LEN_BYTES as isize) {
        let byte = std::ptr::read(ptr.offset(idx));
        size_as_bytes[idx as usize] = byte;
    }

    std::mem::transmute(size_as_bytes)
}
