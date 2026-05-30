use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use earmark_core::{
    ActorId, ClassId, ObjectId, ObjectRecord, PacketId, PacketTemplateId, RunId, RunStatus,
    Standing, SystemId, SystemPackId, TransitionId, VersionId, VersionRecord, WorkflowId,
};
use earmark_index::sqlite_index::SqliteIndex;
#[cfg(feature = "surreal")]
use earmark_index::surreal_index::SurrealIndex;
use earmark_store::file_store::FileStore;
use earmark_store::traits::CanonicalStore;
use earmark_store::traits::DerivedIndex;
use tempfile::TempDir;
use tokio::runtime::Runtime;

fn create_mock_object(i: usize) -> (ObjectRecord, VersionRecord) {
    let obj_id = ObjectId::parse(&format!("obj_bench_{:05}", i)).unwrap();
    let class_id = ClassId::parse("cls_bench").unwrap();
    let version_id = VersionId::parse(&format!("ver_bench_{:05}", i)).unwrap();
    let actor_id = ActorId::parse("act_bench").unwrap();

    let obj = ObjectRecord {
        id: obj_id.clone(),
        class_id: Some(class_id),
        latest_version_id: version_id.clone(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let ver = VersionRecord {
        object_id: obj_id,
        version_id,
        payload: serde_json::Value::Null,
        standing: Standing { dimensions: vec![] },
        signal: None,
        created_at: Utc::now(),
        created_by: Some(actor_id),
    };

    (obj, ver)
}

fn create_mock_run(i: usize) -> earmark_core::RunRecord {
    let run_id = RunId::parse(&format!("run_bench_{:05}", i)).unwrap();
    let workflow_id = WorkflowId::parse("wfl_bench").unwrap();

    earmark_core::RunRecord {
        run_id,
        workflow_id: Some(workflow_id),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        status: RunStatus::Scheduled,
    }
}

fn create_mock_packet(i: usize) -> earmark_core::PacketRecord {
    let packet_id = PacketId::parse(&format!("pkt_bench_{:05}", i)).unwrap();
    let run_id = RunId::parse("run_bench_00000").unwrap();

    earmark_core::PacketRecord {
        packet_id,
        system_pack_ref: SystemPackId::parse("spk_bench").unwrap(),
        system_ref: SystemId::parse("sys_bench").unwrap(),
        run_id,
        workflow_ref: WorkflowId::parse("wfl_bench").unwrap(),
        transition_id: TransitionId::parse("trn_bench").unwrap(),
        packet_template_ref: PacketTemplateId::parse("tpl_bench").unwrap(),
        root_object_ids: vec![],
        included_object_refs: vec![],
        excluded_object_refs: vec![],
        exclusion_reasons: vec![],
        relation_traversal_trace: vec![],
        standing_filter_trace: vec![],
        redaction_trace: vec![],
        provider_exposure_trace: vec![],
        instruction_ref: None,
        protocol_ref: earmark_core::RuntimeProtocolId::parse("prt_bench").unwrap(),
        selection_ref: None,
        provider_profile_ref: None,
        worker_profile_ref: None,
        output_contract_ref: ClassId::parse("cls_bench").unwrap(),
        rendered_manifest: None,
        selection_trace: vec![],
        created_at: Utc::now(),
    }
}

fn bench_index_rebuild(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_store = TempDir::new().unwrap();
    let store = FileStore::new(temp_store.path());
    store.init().unwrap();

    // Pre-populate with 100 objects
    for i in 0..100 {
        let (obj, ver) = create_mock_object(i);
        store.deposit_object(obj, ver).unwrap();
    }

    let mut group = c.benchmark_group("index_rebuild");

    // Benchmark SQLite
    let temp_sqlite = TempDir::new().unwrap();
    let sqlite_path = temp_sqlite.path().join("index.sqlite");
    group.bench_function("sqlite", |b| {
        let mut index = SqliteIndex::open(&sqlite_path).unwrap();
        b.iter(|| {
            rt.block_on(index.rebuild_from_store(&store)).unwrap();
        })
    });

    #[cfg(feature = "surreal")]
    {
        // Benchmark SurrealDB
        let temp_surreal = TempDir::new().unwrap();
        let surreal_path = temp_surreal.path().join("index.surreal");
        group.bench_function("surreal", |b| {
            let mut index = rt.block_on(SurrealIndex::open(&surreal_path)).unwrap();
            b.iter(|| {
                rt.block_on(index.rebuild_from_store(&store)).unwrap();
            })
        });
    }

    group.finish();
}

fn bench_deposit_latency(c: &mut Criterion) {
    let temp_store = TempDir::new().unwrap();
    let store = FileStore::new(temp_store.path());
    store.init().unwrap();

    let mut group = c.benchmark_group("deposit_latency");

    group.bench_function("object_deposit", |b| {
        let mut i = 0;
        b.iter(|| {
            let (obj, ver) = create_mock_object(i);
            store.deposit_object(obj, ver).unwrap();
            i += 1;
            black_box(())
        })
    });

    group.bench_function("run_create", |b| {
        let mut i = 0;
        b.iter(|| {
            let run = create_mock_run(i);
            store.create_run(run).unwrap();
            i += 1;
            black_box(())
        })
    });

    group.bench_function("packet_create", |b| {
        let mut i = 0;
        b.iter(|| {
            let packet = create_mock_packet(i);
            store.create_packet(packet).unwrap();
            i += 1;
            black_box(())
        })
    });

    group.finish();
}

fn bench_read_latency(c: &mut Criterion) {
    let temp_store = TempDir::new().unwrap();
    let store = FileStore::new(temp_store.path());
    store.init().unwrap();

    // Pre-populate
    let (obj, ver) = create_mock_object(0);
    let obj_id = obj.id.clone();
    store.deposit_object(obj, ver).unwrap();

    let mut group = c.benchmark_group("read_latency");

    group.bench_function("get_object", |b| {
        b.iter(|| {
            let _ = store.get_object(&obj_id).unwrap();
            black_box(())
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_index_rebuild,
    bench_deposit_latency,
    bench_read_latency
);
criterion_main!(benches);
