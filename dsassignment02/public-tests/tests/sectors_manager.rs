use assignment_2_solution::{build_sectors_manager, SectorVec};
use ntest::timeout;
use tempfile::tempdir;
use std::path::PathBuf;

#[tokio::test]
#[timeout(200)]
async fn drive_can_store_data() {
    // given
    let root_drive_dir = tempdir().unwrap();
    let mut path = PathBuf::new();
    path.push(root_drive_dir.path());
    // path.push("./omg");
    let sectors_manager = build_sectors_manager(path);

    // when
    sectors_manager
        .write(0, &(SectorVec(vec![2; 4096]), 1, 1))
        .await;
    let data = sectors_manager.read_data(0).await;

    // then
    assert_eq!(sectors_manager.read_metadata(0).await, (1, 1));
    assert_eq!(data.0.len(), 4096);
    assert_eq!(data.0, vec![2; 4096])
}
