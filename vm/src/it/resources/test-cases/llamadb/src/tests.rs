/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! Module with tests.

#[test]
fn integration_sql_test() {
    //
    // Success cases.
    //

    let create_table = execute_sql("CREATE TABLE Users(id INT, name TEXT, age INT)");
    assert_eq!(create_table, "table created");

    let insert_one = execute_sql("INSERT INTO Users VALUES(1, 'Sara', 23)");
    assert_eq!(insert_one, "rows inserted: 1");

    let insert_several =
        execute_sql("INSERT INTO Users VALUES(2, 'Bob', 19), (3, 'Caroline', 31), (4, 'Max', 25)");
    assert_eq!(insert_several, "rows inserted: 3");

    let create_table_role = execute_sql("CREATE TABLE Roles(user_id INT, role VARCHAR(128))");
    assert_eq!(create_table_role, "table created");

    let insert_roles = execute_sql(
        "INSERT INTO Roles VALUES(1, 'Teacher'), (2, 'Student'), (3, 'Scientist'), (4, 'Writer')",
    );
    assert_eq!(insert_roles, "rows inserted: 4");

    let empty_select = execute_sql("SELECT * FROM Users WHERE name = 'unknown'");
    assert_eq!(empty_select, "id, name, age\n");

    let select_all = execute_sql(
        "SELECT min(user_id) as min, max(user_id) as max, \
         count(user_id) as count, sum(user_id) as sum, avg(user_id) as avg FROM Roles",
    );
    assert_eq!(select_all, "min, max, count, sum, avg\n1, 4, 4, 10, 2.5");

    let select_with_join = execute_sql(
        "SELECT u.name AS Name, r.role AS Role FROM Users u JOIN Roles \
         r ON u.id = r.user_id WHERE r.role = 'Writer'",
    );
    assert_eq!(select_with_join, "name, role\nMax, Writer");

    let explain = execute_sql("EXPLAIN SELECT id, name FROM Users");
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
    assert_eq!(delete, "rows deleted: 1");

    let update_query = execute_sql("UPDATE Users SET name = 'Min' WHERE name = 'Max'");
    assert_eq!(update_query, "rows updated: 1");

    //
    // Error cases.
    //

    let empty_str = execute_sql("");
    assert_eq!(empty_str, "[Error] Expected SELECT, INSERT, CREATE, DELETE, TRUNCATE or EXPLAIN statement; got no more tokens");

    let invalid_sql = execute_sql("123");
    assert_eq!(invalid_sql, "[Error] Expected SELECT, INSERT, CREATE, DELETE, TRUNCATE or EXPLAIN statement; got Number(\"123\")");

    let bad_query = execute_sql("SELECT salary FROM Users");
    assert_eq!(bad_query, "[Error] column does not exist: salary");

    let lexer_error = execute_sql("π");
    assert_eq!(lexer_error, "[Error] Lexer error: Unknown character π");

    let incompatible_types = execute_sql("SELECT * FROM Users WHERE age = 'Bob'");
    assert_eq!(
        incompatible_types,
        "[Error] 'Bob' cannot be cast to Integer { signed: true, bytes: 8 }"
    );

    let not_supported_order_by = execute_sql("SELECT * FROM Users ORDER BY name");
    assert_eq!(
        not_supported_order_by,
        "[Error] order by in not implemented"
    );

    let truncate = execute_sql("TRUNCATE TABLE Users");
    assert_eq!(truncate, "rows deleted: 3");

    let drop_table = execute_sql("DROP TABLE Users");
    assert_eq!(drop_table, "table was dropped");

    let select_by_dropped_table = execute_sql("SELECT * FROM Users");
    assert_eq!(
        select_by_dropped_table,
        "[Error] table does not exist: users"
    );
}

//
// Private helper functions.
//
use std::mem;
use std::ptr;

pub unsafe fn read_result_from_mem(ptr: *mut u8) -> Vec<u8> {
    const RESULT_SIZE_BYTES: usize = 4;

    // read string length from current pointer
    let mut len_as_bytes: [u8; RESULT_SIZE_BYTES] = [0; RESULT_SIZE_BYTES];
    ptr::copy_nonoverlapping(ptr, len_as_bytes.as_mut_ptr(), RESULT_SIZE_BYTES);

    let input_len: u32 = mem::transmute(len_as_bytes);
    let total_len = input_len as usize + RESULT_SIZE_BYTES;

    // creates object from raw bytes
    let mut input = Vec::from_raw_parts(ptr, total_len, total_len);

    // drains RESULT_SIZE_BYTES from the beginning of created vector, it allows to effectively
    // skips (without additional allocations) length of the result.
    {
        input.drain(0..4);
    }
    input
}

/// Executes sql and returns result as a String.
fn execute_sql(sql: &str) -> String {
    unsafe {
        use std::mem;

        let mut sql_str = sql.to_string();
        // executes query
        let result_str_ptr = super::invoke(sql_str.as_bytes_mut().as_mut_ptr(), sql_str.len());
        // ownership of memory of 'sql_str' will be taken through 'ptr' inside
        // 'invoke' method, have to forget 'sql_str'  for prevent deallocation
        mem::forget(sql_str);
        // converts results
        let result_str = read_result_from_mem(result_str_ptr.as_ptr());
        String::from_utf8(result_str).unwrap()
    }
}
