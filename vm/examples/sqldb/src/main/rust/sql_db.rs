
use std::sync::Mutex;
use std::vec::Vec;

//
// Private Implementation
//

#[derive(Clone)]
struct Table<T> {
    rows: Vec<Vec<T>>
}

impl<T> Table<T> {
    fn build(rows: Vec<Vec<T>>) -> Self { Table { rows } }
}

#[derive(Clone)]
struct Cursor {
    row: T,
    col: T
}

impl Cursor {
    fn new() -> Self { Cursor { row: -1, col: -1 } }
    fn inc_row(&mut self) { self.row += 1 }
    fn inc_col(&mut self) { self.col += 1 }
    fn reset_col(&mut self) { self.col = -1 }
    /// Increases row index by one and reset col index
    fn select_next_row(&mut self) {
        self.inc_row();
        self.reset_col()
    }
}

//
// Public API
//

pub type T  = i32;

pub struct Db<T> {
    table: Table<T>,
    view: Table<T>,
    cursor: Cursor
}

impl Db<T> {

    fn for_table(table: Table<T>) -> Self {
        Db { table, view: Table::build(Vec::new()), cursor: Cursor::new() }
    }

    /// Resets the cursor
    fn reset_cursor(&mut self) {
        self.cursor = Cursor::new()
    }

    /// Resets the view
    fn clean_view(&mut self) {
        self.view.rows.clear()
    }

    /// Remove all effects from previous query.
    /// New query remove dirty state from previous query.
    fn clean_db_state(&mut self) {
        self.reset_cursor();
        self.clean_view();
    }

    /// Returns row by row id.
    fn get_row(&self, row_id: usize) -> Option<&Vec<T>> {
        self.view.rows.get(row_id)
    }

    /// Returns record by row id and column id.
    fn get_rec(&self, row_id: usize, col_id: usize) -> Option<&T> {
        self.get_row(row_id).and_then(|row| row.get(col_id))
    }

    /// Returns current record according to the cursor
    fn get_current_field(&self) -> Option<&T> {
        self.get_rec(self.cursor.row as usize, self.cursor.col as usize)
    }

    /// Returns current row id according to the cursor
    fn get_current_row(& self) -> Option<&Vec<T>> {
        self.get_row(self.cursor.row as usize)
    }

    /// Return filtered copy of current Table
    fn filter_table(&self, field_idx: usize, field_val: T) -> Table<T> {
        Table::build(
            self.table.rows.iter()
                .filter(|row| row[field_idx] == field_val)
                .cloned()
                .collect::<Vec<Vec<T>>>()
        )
    }

    //
    // Public select function
    //

    /// `select * from Table`. Creates cursor in memory.
    pub fn set_query_wildcard(&mut self) {
        // clean db state (next query abort previous not completed query)
        self.clean_db_state();
        // copy the table into the view as is
        self.view = self.table.clone();
    }

    /// `select * from Table where {FIELD} = {VALUE}`. Creates cursor in memory.
    ///
    /// # Arguments
    ///
    /// * field_idx - The index of field that will be compared
    /// * field_val - Searched value of specified field
    ///
    pub fn set_query_wildcard_where(&mut self, field_idx: usize, field_val: T) {
        // clean db state (next query abort previous not completed query)
        self.clean_db_state();

        self.view = self.filter_table(field_idx, field_val);
    }

    /// `select count(*) from Table`. Creates cursor in memory.
    pub fn set_count_query(&mut self) {
        self.clean_db_state();

        self.view = Table::build(vec![vec![self.table.rows.len() as i32]]);
    }

    /// `select count(*) from Table where {FIELD} = {VALUE}`. Creates cursor in memory.
    ///
    /// # Arguments
    ///
    /// * field_idx - The index of field that will be compared
    /// * field_val - Searched value of specified field
    ///
    pub fn set_count_query_where(&mut self, field_idx: usize, field_val: T) {
        self.clean_db_state();

        let amount = self.filter_table(field_idx, field_val).rows.len();
        self.view = Table::build(vec![vec![amount as i32]]);
    }

    /// `select avg({FIELD}) from Table`. Creates cursor in memory.
    ///
    /// # Arguments
    ///
    /// * avg_field_idx - The index of field for which will calculate average
    pub fn set_average_query(&mut self, avg_field_idx: usize) {
        self.clean_db_state();

        let sum: i32 =
            self.table.rows
                .iter()
                .map(|row| { row[avg_field_idx] })
                .sum();
        let avg: f32 = sum as f32 / self.table.rows.len() as f32;

        // put result in a table as int
        self.view = Table::build(vec![vec![avg as i32]]); // todo return float
    }

    /// `select count(*) from Table where {FIELD} = {VALUE}`. Creates cursor in memory.
    ///
    /// # Arguments
    ///
    /// * avg_field_idx - Average will calculated for field with this index
    /// * field_idx - The index of field that will be compared
    /// * field_val - Searched value of specified field
    ///
    pub fn set_average_query_where(
        &mut self,
        avg_field_idx: usize,
        field_idx: usize,
        field_val: T
    ) {
        self.clean_db_state();

        let filtered_rows = self.filter_table(field_idx, field_val).rows;
        let sum: i32 =
            filtered_rows
                .iter()
                .map(|row| { row[avg_field_idx] })
                .sum();
        let avg: f32 = sum as f32 / filtered_rows.len() as f32;

        // put result in a table as int
        self.view = Table::build(vec![vec![avg as i32]]); // todo return float
    }

    //
    // Cursor management
    //

    /// Returns an serial number of the next row or -1 otherwise
    pub fn next_row(&mut self) -> i32 {
        self.cursor.select_next_row();
        // if row exists return its serial number or -1 otherwise
        self.get_current_row().map(|_| self.cursor.row).unwrap_or(-1)
    }

    /// Returns a next field value for the current row
    pub fn next_field(&mut self) -> i32 {
        self.cursor.inc_col();
        *self.get_current_field().unwrap_or(&-1)
    }

}

lazy_static! {

    pub static ref DB: Mutex<Db<T>> = Mutex::new(Db::for_table(
        Table::build(
            vec![
                // table with crypto currencies
                // id, symbol, price, moth
                vec![0, 0, 6500, 10],
                vec![1, 1, 450,  10],
                vec![2, 2, 100,  10],
                vec![3, 0, 7000, 11],
                vec![4, 1, 500,  11],
                vec![5, 2, 80,   11],
                vec![6, 1, 6700, 12],
                vec![7, 2, 400,  12],
                vec![8, 3, 120,  12]
            ]
        )
    ));

}
