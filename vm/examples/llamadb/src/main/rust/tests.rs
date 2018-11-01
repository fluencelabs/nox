//! Module with tests.

use fluence::memory::STR_LEN_BYTES;

extern crate fluence_sdk as fluence;

#[test]
fn integration_sql_test() {
    //
    // Success cases.
    //

    let create_table = execute_sql("CREATE TABLE Users(id INT, name TEXT, age INT)");
    println!("{}", create_table);
    assert_eq!(create_table, "table created");

    let insert_one = execute_sql("INSERT INTO Users VALUES(1, 'Sara', 23)");
    println!("{}", insert_one);
    assert_eq!(insert_one, "rows inserted: 1");

    let insert_several =
        execute_sql("INSERT INTO Users VALUES(2, 'Bob', 19), (3, 'Caroline', 31), (4, 'Max', 25)");
    println!("{}", insert_several);
    assert_eq!(insert_several, "rows inserted: 3");

    let create_table_role = execute_sql("CREATE TABLE Roles(user_id INT, role VARCHAR(128))");
    println!("{}", create_table_role);
    assert_eq!(create_table_role, "table created");

    let insert_roles = execute_sql(
        "INSERT INTO Roles VALUES(1, 'Teacher'), (2, 'Student'), (3, 'Scientist'), (4, 'Writer')",
    );
    println!("{}", insert_roles);
    assert_eq!(insert_roles, "rows inserted: 4");

    let empty_select = execute_sql("SELECT * FROM Users WHERE name = 'unknown'");
    println!("{}", empty_select);
    assert_eq!(empty_select, "id, name, age\n");

    let select_all = execute_sql(
        "SELECT min(user_id) as min, max(user_id) as max, \
         count(user_id) as count, sum(user_id) as sum, avg(user_id) as avg FROM Roles",
    );
    assert_eq!(select_all, "min, max, count, sum, avg\n1, 4, 4, 10, 2.5");
    println!("{}", select_all);

    let select_with_join = execute_sql(
        "SELECT u.name AS Name, r.role AS Role FROM Users u JOIN Roles \
         r ON u.id = r.user_id WHERE r.role = 'Writer'",
    );
    println!("{}", select_with_join);
    assert_eq!(select_with_join, "name, role\nMax, Writer");

    let explain = execute_sql("EXPLAIN SELECT id, name FROM Users");
    println!("{}", explain);
    assert_eq!(
        explain,
        "query plan\n".to_string()
            + "column names: (`id`, `name`)\n"
            + "(scan `users` :source-id 0\n"
            + "  (yield\n"
            + "    (column-field :source-id 0 :column-offset 0)\n"
            + "    (column-field :source-id 0 :column-offset 1)))"
    );

    let delete = execute_sql(
        "DELETE FROM Users WHERE id = (SELECT user_id FROM Roles WHERE role = 'Student');",
    );
    println!("{}", delete);
    assert_eq!(delete, "rows deleted: 1");

    let update_query = execute_sql("UPDATE Users SET name = 'Min' WHERE name = 'Max'");
    println!("{}", update_query);
    assert_eq!(update_query, "rows updated: 1");

    //
    // Error cases.
    //

    let empty_str = execute_sql("");
    println!("{}", empty_str);
    assert_eq!(empty_str, "[Error] Expected SELECT, INSERT, CREATE, DELETE, TRUNCATE or EXPLAIN statement; got no more tokens");

    let invalid_sql = execute_sql("123");
    println!("{}", invalid_sql);
    assert_eq!(invalid_sql, "[Error] Expected SELECT, INSERT, CREATE, DELETE, TRUNCATE or EXPLAIN statement; got Number(\"123\")");

    let bad_query = execute_sql("SELECT salary FROM Users");
    println!("{}", bad_query);
    assert_eq!(bad_query, "[Error] column does not exist: salary");

    let lexer_error = execute_sql("π");
    println!("{}", lexer_error);
    assert_eq!(lexer_error, "[Error] Lexer error: Unknown character π");

    let incompatible_types = execute_sql("SELECT * FROM Users WHERE age = 'Bob'");
    println!("{}", incompatible_types);
    assert_eq!(
        incompatible_types,
        "[Error] 'Bob' cannot be cast to Integer { signed: true, bytes: 8 }"
    );

    let not_supported_order_by = execute_sql("SELECT * FROM Users ORDER BY name");
    println!("{}", not_supported_order_by);
    assert_eq!(
        not_supported_order_by,
        "[Error] order by in not implemented"
    );

    let truncate = execute_sql("TRUNCATE TABLE Users");
    println!("{}", truncate);
    assert_eq!(truncate, "rows deleted: 3");

    let drop_table = execute_sql("DROP TABLE Users");
    println!("{}", drop_table);
    assert_eq!(drop_table, "table was dropped");

    let select_by_dropped_table = execute_sql("SELECT * FROM Users");
    println!("{}", select_by_dropped_table);
    assert_eq!(
        select_by_dropped_table,
        "[Error] table does not exist: users"
    );
}

//
// Private helper functions.
//

/// Executes sql and returns result as a String.
fn execute_sql(sql: &str) -> String {
    unsafe {
        // converts params
        let str_len = sql.len();
        let sql_str_ptr = fluence::memory::write_str_to_mem(sql).unwrap();

        // executes query
        let result_str_ptr = super::do_query(sql_str_ptr.as_ptr().add(STR_LEN_BYTES), str_len);

        // converts results
        fluence::memory::read_str_from_fat_ptr(result_str_ptr).unwrap()
    }
}
