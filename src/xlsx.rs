use std::path::Path;

use diesel::{QueryDsl, RunQueryDsl, SelectableHelper, SqliteConnection};
use rust_xlsxwriter::{Format, Workbook};
use serde::Serialize;

mod types;

pub fn write(path: impl AsRef<Path>, mut db: SqliteConnection) -> crate::Result<()> {
    log::info!("Writing DB to XLSX file...");

    let mut writer = {
        let workbook = Workbook::new();

        let bold_format = Format::new().set_bold();

        XlsxWriter {
            workbook,
            bold_format,
        }
    };

    let lines = lumberjack_parse::schema::lines::table
        .select(lumberjack_parse::data::Line::as_select())
        .load(&mut db)?;
    let lines: Vec<types::Line> = lines.into_iter().map(types::Line::from).collect();
    writer.write_worksheet_serializable("Lines", &lines)?;

    let objects = lumberjack_parse::schema::objects::table
        .select(lumberjack_parse::data::Object::as_select())
        .load(&mut db)?;
    let objects: Vec<types::Object> = objects.into_iter().map(types::Object::from).collect();
    writer.write_worksheet_serializable("Objects", &objects)?;

    let files = lumberjack_parse::schema::files::table
        .select(lumberjack_parse::data::File::as_select())
        .load(&mut db)?;
    let files: Vec<types::File> = files.into_iter().map(types::File::from).collect();
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
