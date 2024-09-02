use std::{
    rc::Rc,
    sync::Once,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::types::Value;

/// Create logs with the given data, run the parser, and return a connection to the resulting database.
fn test_with_data<F>(data: &str, f: F)
where
    F: FnOnce(rusqlite::Connection),
{
    static INIT_LOGGING: Once = Once::new();

    INIT_LOGGING.call_once(|| {
        env_logger::builder().init();
    });

    let temp_dir = std::env::temp_dir().join(format!("lumberjack_test_queries_{}/", epoch_id()));
    std::fs::remove_dir_all(&temp_dir).ok();
    std::fs::create_dir(&temp_dir).ok();
    let logs_path = temp_dir.join("test.cbllog");

    std::fs::write(&logs_path, data).unwrap();

    let db_path = temp_dir.join("output.sqlite");

    lumberjack_parse::parse(&logs_path, &db_path, lumberjack_parse::Options::default())
        .expect("Parsing failed!");

    let conn = rusqlite::Connection::open(db_path).expect("Failed to open database");
    rusqlite::vtab::array::load_module(&conn).expect("Failed to load array module");

    f(conn);

    if std::env::var("LUMBERJACK_TEST_KEEP").is_err() {
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}

// Returns a unique (within the same process) identifier every time it is called. Useful to run tests in parallel.
fn epoch_id() -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    lazy_static::lazy_static! {
        static ref EPOCH_COUNTER: AtomicU32 = AtomicU32::new(0);
    }

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let counter = EPOCH_COUNTER.fetch_add(1, Ordering::AcqRel);

    format!("{}{}{}", now.as_secs(), now.subsec_micros(), counter)
}

/// Find all of the revisions which were pulled from Sync Gateway but not inserted to the database
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

    test_with_data(TEST_DATA, |conn| {
        // Get all pairs of doc_id and rev_id for all doc_ids which appear in an incoming_rev_received event but not in a db_saved_rev event
        let mut statement = conn
            .prepare(
                "
                WITH incoming_revs(doc_id, rev_id) AS (
                    SELECT
                        json_extract(lines.event_data, '$.doc_id'),
                        json_extract(lines.event_data, '$.rev_id')
                    FROM lines
                    WHERE event_type = (SELECT id FROM event_types WHERE name = 'IncomingrevReceived')
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
    });
}

/// Find the Replicator Correlation IDs where a given rev was synced (either push or pull).
/// TODO: LiteCore 3.2.0 does not currently log pushed revisions, so this test only checks for pulled revisions.
#[test]
fn find_synced_rev() {
    const TEST_DATA: &str = concat!(
        "---- CouchbaseLite/3.2.0 (.NET; Microsoft Windows 10.0.22621) Build/1 LiteCore/3.2.0 (1) Commit/86734653b94fa6db+7f0707145d9db2af ----\n",
        "2024-08-19T12:46:35.661486 Sync Info Obj=/Repl#50/ CorrID=5b2affd2 Received X-Correlation-Id\n",
        "2024-08-19T12:46:35.661486 Sync Info Obj=/Repl#51/ CorrID=1c05e6b1 Received X-Correlation-Id\n",
        "2023-12-08T23:39:23.252743 Sync Verbose Obj=/Repl#50/Puller#52/IncomingRev#65/ Coll=0 Received revision 'project::9243bc22-9576-4e38-815f-6ee47e3d9032' #2-d57dc7e01da7cc97c114f919c10553cd (seq '\"18074:394\"')\n",
        "2023-12-08T23:39:23.253369 Sync Verbose Obj=/Repl#51/Puller#53/IncomingRev#66/ Coll=0 Received revision 'administrativegroup::10c751a7-105f-4c3f-9970-97fe5607a4fb' #1-60c2473c82d69822de6eb1737d563168 (seq '\"18074:432\"')\n",
    );

    const REV_ID: &str = "2-d57dc7e01da7cc97c114f919c10553cd";

    test_with_data(TEST_DATA, |conn| {
        let object_paths: Vec<String> = conn
            .prepare(
                "
                SELECT lines.object_path
                FROM lines
                WHERE
                    lines.event_type = (SELECT id FROM event_types WHERE name = 'IncomingrevReceived')
                    AND (SELECT json_extract(lines.event_data, '$.rev_id')) = ?
            ",
            )
            .unwrap()
            .query_map([REV_ID], |row| Ok(row.get::<_, String>(0).unwrap()))
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        let repl_paths: Vec<String> = object_paths
            .into_iter()
            .map(|path| {
                let count = path.split('/').count();
                path.split('/')
                    .take(count - 2)
                    .collect::<Vec<&str>>()
                    .join("/")
            })
            .collect();

        // Weird magic we have to do to pass a vec as a parameter to a query
        let parent_paths = Rc::new(
            repl_paths
                .into_iter()
                .map(Value::from)
                .collect::<Vec<Value>>(),
        );

        let results: Vec<String> = conn.prepare(
            "
            SELECT json_extract(lines.event_data, '$.correlation_id')
            FROM lines
            WHERE
                lines.object_path IN rarray(?)
                AND lines.event_type = (SELECT id FROM event_types WHERE name = 'ReplReceivedCorrelationId')
        ",
        ).unwrap()
        .query_map([parent_paths], |row| Ok(row.get::<_, String>(0).unwrap()))
        .unwrap()
        .filter_map(Result::ok)
        .collect();

        let expected_results = vec!["5b2affd2".to_string()];

        assert_eq!(expected_results, results);
    });
}

/// Find the reason for a rev not being pushed to Sync Gateway.
#[test]
fn find_failed_pushes() {
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
    struct RevFailures {
        obsolete: bool,
        proposed_conflict: bool,
        rev_conflict: bool,
        invalid_ancestor: bool,
        error_response: bool,
        read_failed: bool,
    }

    const TEST_DATA: &str = concat!(
        "---- CouchbaseLite/3.2.0 (.NET; Microsoft Windows 10.0.22621) Build/1 LiteCore/3.2.0 (1) Commit/86734653b94fa6db+7f0707145d9db2af ----\n",
        "2023-12-08T23:39:23.252743 Sync Verbose Obj=/Repl#52/Pusher#76/ Coll=0 Revision 'mydoc123' #1-60c2473c82d69822de6eb1737d563168 is obsolete; not sending it\n",
        "2023-12-08T23:39:38.564871 Sync Verbose Obj=/Repl#52/Pusher#76/ Coll=0 Proposed rev 'accounts773' #3-d57dc7e01da7cc97c114f919c10553cd (ancestor #2-e9f91077c5126dd7f5bd464ea8b8d7d3) conflicts with server revision (#3-bca3778f342fe8f57ad708893b181bd6)\n",
        "2023-12-08T23:42:11.798165 Sync Verbose Obj=/Repl#52/Pusher#76/ Coll=0 Rev 'customer7b6d' #2-df1818945ea9b968eb49699159950c7b conflicts with newer server revision\n",
        "2023-12-08T23:43:18.189150 Sync Verbose Obj=/Repl#52/Pusher#76/ Coll=0 Proposed rev 'accounts195' #6-bdccb8fb5edd4640001e42c6dc7bf1c8 has invalid ancestor 8-3d834cf9b9ed4b48ce5c5d64279f3ec5\n",
        "2023-12-08T23:45:54.968741 Sync Verbose Obj=/Repl#52/Pusher#76/ Coll=0 Got error response to rev 'customer985c' #1-36c17445434db7cac57b84b3373c9b01 (seq #841): HTTP 403 'read_only'\n",
        "2023-12-08T23:45:54.968741 Sync Verbose Obj=/Repl#52/Pusher#76/ sendRevision: Couldn't get rev 'customer58ba' 5-df45ce2889f7e94226a36beb6754c350 from db: LiteCore CryptoError, \"encryption/decryption error\"\n",
    );

    let expected_results: [(&str, RevFailures); 6] = [
        (
            "1-60c2473c82d69822de6eb1737d563168",
            RevFailures {
                obsolete: true,
                ..Default::default()
            },
        ),
        (
            "3-d57dc7e01da7cc97c114f919c10553cd",
            RevFailures {
                proposed_conflict: true,
                ..Default::default()
            },
        ),
        (
            "2-df1818945ea9b968eb49699159950c7b",
            RevFailures {
                rev_conflict: true,
                ..Default::default()
            },
        ),
        (
            "6-bdccb8fb5edd4640001e42c6dc7bf1c8",
            RevFailures {
                invalid_ancestor: true,
                ..Default::default()
            },
        ),
        (
            "1-36c17445434db7cac57b84b3373c9b01",
            RevFailures {
                error_response: true,
                ..Default::default()
            },
        ),
        (
            "5-df45ce2889f7e94226a36beb6754c350",
            RevFailures {
                read_failed: true,
                ..Default::default()
            },
        ),
    ];

    test_with_data(TEST_DATA, |conn| {
        let get_failure = |rev_id: &str| {
            conn.prepare("
                        WITH obsolete_revs(x) AS (
                            SELECT 1 FROM lines
                            WHERE lines.event_type = (SELECT id FROM event_types WHERE name = 'PusherSkipObsolete')
                                AND json_extract(lines.event_data, '$.rev_id') = ?1
                        ),
                        proposed_conflicts(x) AS (
                            SELECT 1 FROM lines
                            WHERE lines.event_type = (SELECT id FROM event_types WHERE name = 'PusherProposedConflict')
                                AND json_extract(lines.event_data, '$.rev_id') = ?1
                        ),
                        rev_conflicts(x) AS (
                            SELECT 1 FROM lines
                            WHERE lines.event_type = (SELECT id FROM event_types WHERE name = 'PusherRevConflict')
                                AND json_extract(lines.event_data, '$.rev_id') = ?1
                        ),
                        invalid_ancestors(x) AS (
                            SELECT 1 FROM lines
                            WHERE lines.event_type = (SELECT id FROM event_types WHERE name = 'PusherProposedInvalidAncestor')
                                AND json_extract(lines.event_data, '$.rev_id') = ?1
                        ),
                        error_responses(x) AS (
                            SELECT 1 FROM lines
                            WHERE lines.event_type = (SELECT id FROM event_types WHERE name = 'PusherGotErrorResponse')
                                AND json_extract(lines.event_data, '$.rev_id') = ?1
                        ),
                        read_failures(x) AS (
                            SELECT 1 FROM lines
                            WHERE lines.event_type = (SELECT id FROM event_types WHERE name = 'PusherReadFailed')
                                AND json_extract(lines.event_data, '$.rev_id') = ?1
                        )
                        SELECT EXISTS(SELECT 1 FROM obsolete_revs) AS obsolete,
                               EXISTS(SELECT 1 FROM proposed_conflicts) AS proposed_conflict,
                               EXISTS(SELECT 1 FROM rev_conflicts) AS rev_conflict,
                               EXISTS(SELECT 1 FROM invalid_ancestors) AS invalid_ancestor,
                               EXISTS(SELECT 1 FROM error_responses) AS error_response,
                               EXISTS(SELECT 1 FROM read_failures) AS read_failed
                        ").unwrap().query_map([rev_id], |row| {
                            Ok(RevFailures {
                                obsolete: row.get("obsolete")?,
                                proposed_conflict: row.get("proposed_conflict")?,
                                rev_conflict: row.get("rev_conflict")?,
                                invalid_ancestor: row.get("invalid_ancestor")?,
                                error_response: row.get("error_response")?,
                                read_failed: row.get("read_failed")?,
                            })
                        }).unwrap().filter_map(Result::ok).collect()
        };

        for (rev_id, expected) in expected_results {
            let results: Vec<RevFailures> = get_failure(rev_id);
            assert_eq!(results[0], expected);
        }
    });
}
