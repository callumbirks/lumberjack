#[test]
fn find_uninserted_revs() {
    const TEST_DATA: &str = concat!(
        "---- CouchbaseLite/3.2.0 (.NET; Microsoft Windows 10.0.22621) Build/1 LiteCore/3.2.0 (1) Commit/86734653b94fa6db+7f0707145d9db2af ----\n",
        "2023-12-08T23:39:23.252743 Sync Verbose Obj=/IncomingRev#106/ Coll=0 Received revision 'project::9243bc22-9576-4e38-815f-6ee47e3d9032' #2-d57dc7e01da7cc97c114f919c10553cd (seq '\"18074:394\"')\n",
        "2023-12-08T23:39:23.253369 Sync Verbose Obj=/IncomingRev#107/ Coll=0 Received revision 'administrativegroup::10c751a7-105f-4c3f-9970-97fe5607a4fb' #1-60c2473c82d69822de6eb1737d563168 (seq '\"18074:432\"')\n",
        "2023-12-08T23:39:23.253493 Sync Verbose Obj=/IncomingRev#108/ Coll=0 Received revision 'project::b2d44c1c-1dd1-4f49-a939-99cbeb388dfc' #2-e9f91077c5126dd7f5bd464ea8b8d7d3 (seq '\"18074:503\"')\n",
        "2023-12-08T23:39:23.253567 Sync Verbose Obj=/IncomingRev#109/ Coll=0 Received revision 'projectcoordinatorstatistics::923a1bd3-f9a6-4621-8feb-e39651bad366' #26-bca3778f342fe8f57ad708893b181bd6 (seq '\"18074:910\"')\n",
        "2023-12-08T23:39:23.276472 DB Verbose Obj=/DB#101/ Saved 'project::9243bc22-9576-4e38-815f-6ee47e3d9032' rev #2-d57dc7e01da7cc97c114f919c10553cd as seq 22\n",
        "2023-12-08T23:39:23.276492 Sync Verbose Obj=/Inserter#100/    {'project::9243bc22-9576-4e38-815f-6ee47e3d9032 (_default)' #2-d57dc7e01da7cc97c114f919c10553cd <- 1-ddcf061cb80d06141f3642c80a856695} seq 22\n",
        "2023-12-08T23:39:23.276713 DB Verbose Obj=/DB#101/ Saved 'administrativegroup::10c751a7-105f-4c3f-9970-97fe5607a4fb' rev #1-60c2473c82d69822de6eb1737d563168 as seq 23\n",
        "2023-12-08T23:39:23.276731 Sync Verbose Obj=/Inserter#100/    {'administrativegroup::10c751a7-105f-4c3f-9970-97fe5607a4fb (_default)' #1-60c2473c82d69822de6eb1737d563168 <- } seq 23\n",
    );
    // 'project::b2d44c1c-1dd1-4f49-a939-99cbeb388dfc' #2-e9f91077c5126dd7f5bd464ea8b8d7d3 is not saved
    // 'projectcoordinatorstatistics::923a1bd3-f9a6-4621-8feb-e39651bad366' #26-bca3778f342fe8f57ad708893b181bd6 is not saved

    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .init();

    let temp_dir = std::env::temp_dir().join("lumberjack_query_test/");
    std::fs::remove_dir(&temp_dir).ok();
    std::fs::create_dir(&temp_dir).ok();
    let logs_path = temp_dir.join("cbl_verbose_1702053551007.cbllog");

    std::fs::write(&logs_path, TEST_DATA).unwrap();

    let db_path = temp_dir.join("output.sqlite");

    lumberjack_parse::parse(&logs_path, &db_path, lumberjack_parse::Options::default())
        .expect("Parsing failed!");

    let conn = rusqlite::Connection::open(db_path).expect("Failed to open database");

    // Get all pairs of doc_id and rev_id for all doc_ids which appear in an incoming_rev_received event but not in a db_saved_rev event
    let mut statement = conn
        .prepare(
            "
            WITH incoming_revs(doc_id, rev_id) AS (
                SELECT
                    json_extract(lines.event_data, '$.doc_id'),
                    json_extract(lines.event_data, '$.rev_id')
                FROM lines
                WHERE event_type = (SELECT id FROM event_types WHERE name = 'IncomingRevReceived')
            ),
            saved_revs(doc_id, rev_id) AS (
                SELECT
                    json_extract(lines.event_data, '$.doc_id'),
                    json_extract(lines.event_data, '$.rev_id')
                FROM lines
                WHERE event_type = (SELECT id FROM event_types WHERE name = 'DbSavedRev')
            )
            SELECT ir.doc_id AS doc_id, ir.rev_id AS rev_id
            FROM incoming_revs ir
            LEFT JOIN saved_revs sr ON ir.doc_id = sr.doc_id
            WHERE sr.doc_id IS NULL AND ir.doc_id IS NOT NULL
        ",
        )
        .unwrap();

    let results: Vec<(String, String)> = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>("doc_id").unwrap(),
                row.get::<_, String>("rev_id").unwrap(),
            ))
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect();

    let expected_results = vec![
        (
            "project::b2d44c1c-1dd1-4f49-a939-99cbeb388dfc".to_string(),
            "2-e9f91077c5126dd7f5bd464ea8b8d7d3".to_string(),
        ),
        (
            "projectcoordinatorstatistics::923a1bd3-f9a6-4621-8feb-e39651bad366".to_string(),
            "26-bca3778f342fe8f57ad708893b181bd6".to_string(),
        ),
    ];

    assert_eq!(expected_results, results);
}
