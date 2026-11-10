//! A file a person brings is often skinned: read at rest, it stands as its
//! joints place it. The default avatars are such files.

use std::path::PathBuf;

use glb::{Glb, Model};

#[test]
fn every_default_avatar_stands_at_rest_as_a_model() {
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/avatars");
    let mut read = 0;
    for entry in std::fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "vrm") {
            continue;
        }
        let glb = Glb::open(&std::fs::read(&path).unwrap()).unwrap();
        let weight = glb.weigh();
        let model = Model::read(&glb).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let tall_m = model.max[1] - model.min[1];
        assert!(
            (0.5..3.0).contains(&tall_m),
            "{}: {tall_m} m",
            path.display()
        );
        // Feet on the ground, near enough: at rest, a humanoid stands.
        assert!(
            model.min[1].abs() < 0.2,
            "{}: {:?}",
            path.display(),
            model.min
        );
        // A weight is never less than what is drawn.
        assert!(weight.triangles >= (model.indices.len() / 3) as u64);
        assert!(!model.pictures.is_empty(), "{}", path.display());
        let texels: u64 = model
            .pictures
            .iter()
            .map(|p| u64::from(p.width) * u64::from(p.height))
            .sum();
        assert!(weight.texels >= texels, "{}", path.display());
        read += 1;
    }
    assert!(read > 0);
}
