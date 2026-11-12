use super::*;
#[test]
fn late_results_and_same_region_new_request_cannot_replace_scene() {
    let mut active = ActiveMapRegion::default();
    let region = super::super::tests::region();
    active.confirm(region).unwrap();
    let key = RequestKey {
        request_id: 1,
        region_revision: 1,
        region,
    };
    let (send, recv) = mpsc::channel();
    let mut builder = MapPackageBuilder {
        worker: Some(Worker {
            pending: Arc::new((Mutex::new(Pending::default()), Condvar::new())),
            results: Mutex::new(recv),
        }),
        current: Some(key),
        job: None,
        serial: 0,
        loading: false,
    };
    send.send((
        RequestKey {
            request_id: 0,
            ..key
        },
        Err("stale".into()),
    ))
    .unwrap();
    assert!(builder.poll(&active).is_none());
    active.confirm(region).unwrap();
    send.send((key, Err("old revision".into()))).unwrap();
    assert!(builder.poll(&active).is_none());
    assert!(builder.current.is_none());
    let key = RequestKey {
        request_id: 2,
        region_revision: 2,
        region,
    };
    builder.current = Some(key);
    send.send((key, Err("current error".into()))).unwrap();
    assert_eq!(
        builder.poll(&active).unwrap().err().as_deref(),
        Some("current error")
    );
}
