fn main() {
    unsafe { sqlite_vec::sqlite3_vec_init(); }
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let row: String = conn.query_row("SELECT vec_version()", [], |r| r.get(0)).unwrap();
    println!("Version: {}", row);
}
