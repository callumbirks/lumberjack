use std::path::Path;

use lumberjack_parse::data::FromRow;
use rust_xlsxwriter::{Format, Workbook};
use serde::Serialize;

mod types;

pub fn write(path: impl AsRef<Path>, db: rusqlite::Connection) -> crate::Result<()> {
    log::info!("Writing DB to XLSX file...");

    let mut writer = {
        let workbook = Workbook::new();

        let bold_format = Format::new().set_bold();

        XlsxWriter {
            workbook,
            bold_format,
        }
    };

    // Convert it to a custom Line type that has more sensible serialization for XLSX
    let lines: Vec<types::Line> = db
        .prepare("SELECT * FROM lines")
        .unwrap()
        .query_map([], lumberjack_parse::data::Line::from_row)?
        .filter_map(Result::ok)
        .map(types::Line::from)
        .collect();

    writer.write_worksheet_serializable("Lines", &lines)?;

    let files: Vec<types::File> = db
        .prepare("SELECT * FROM files")
        .unwrap()
        .query_map([], lumberjack_parse::data::File::from_row)?
        .filter_map(Result::ok)
        .map(types::File::from)
        .collect();

    writer.write_worksheet_serializable("Files", &files)?;

    let path_str = path.as_ref().to_string_lossy();
    writer.save(&path_str)?;
    log::info!("Saved XLSX file to \"{}\"", &path_str);
    Ok(())
}

struct XlsxWriter {
    workbook: Workbook,
    bold_format: Format,
}

impl XlsxWriter {
    fn write_worksheet_serializable<T: Serialize>(
        &mut self,
        name: &str,
        objects: &[T],
    ) -> crate::Result<()> {
        let worksheet = self.workbook.add_worksheet();
        worksheet.set_name(name)?;
        let Some(first) = objects.first() else {
            return Ok(());
        };
        worksheet.serialize_headers_with_format(0, 0, first, &self.bold_format)?;

        for obj in objects {
            worksheet.serialize(&obj)?;
        }

        Ok(())
    }

    fn save(&mut self, name: &str) -> crate::Result<()> {
        self.workbook.save(name).map_err(crate::Error::Xlsx)
    }
}
