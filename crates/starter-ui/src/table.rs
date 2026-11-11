use egui::{Response, Ui};
use egui_table::{AutoSizeMode, Column, HeaderRow, Table, TableDelegate};

pub fn show_table(
    ui: &mut Ui,
    id: &'static str,
    rows: u64,
    columns: Vec<Column>,
    delegate: &mut dyn TableDelegate,
) -> Response {
    Table::new()
        .id_salt(id)
        .num_rows(rows)
        .columns(columns)
        .num_sticky_cols(1)
        .headers(vec![HeaderRow::new(25.0)])
        .auto_size_mode(AutoSizeMode::OnParentResize)
        .show(ui, delegate)
}
