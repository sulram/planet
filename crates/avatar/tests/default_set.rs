//! The committed default set must load: every avatar, every clip.

use std::path::PathBuf;

use avatar::{Avatar, Clip};

fn assets() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn files(folder: &str, extension: &str) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(assets().join(folder))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == extension))
        .collect();
    found.sort();
    assert!(!found.is_empty(), "no {extension} files in assets/{folder}");
    found
}

#[test]
fn every_avatar_loads_and_poses() {
    let idle = Clip::from_glb(&std::fs::read(assets().join("clips/idle.glb")).unwrap()).unwrap();
    for path in files("avatars", "vrm") {
        let avatar = Avatar::from_vrm(&std::fs::read(&path).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(
            !avatar.mesh.vertices.is_empty() && !avatar.mesh.images.is_empty(),
            "{path:?}"
        );
        let joints = avatar.joint_matrices(&idle.sample(0.3));
        assert!(joints.iter().all(|m| m.is_finite()), "{path:?}");
        // Tall or short, a body stands somewhere between a bee and a giant.
        let height = avatar.height_m();
        assert!((0.3..4.0).contains(&height), "{path:?} stands {height} m");
        println!("{} stands {height:.2} m", path.display());
    }
}

#[test]
fn every_clip_loads() {
    for path in files("clips", "glb") {
        let clip = Clip::from_glb(&std::fs::read(&path).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(clip.duration_s > 0.1, "{path:?}");
        let pose = clip.sample(clip.duration_s / 2.0);
        assert!(pose.rotations.iter().flatten().count() > 15, "{path:?}");
    }
}
