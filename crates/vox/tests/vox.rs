//! A `.vox` written here byte by byte, read back, placed and meshed. And the
//! files MagicaVoxel itself wrote, when the reference checkouts are beside
//! the repo (`refs/`, gitignored): they say nothing when they are not.

use vox::{Error, SIDE, default_palette, mesh, parse};

/// One chunk: id, content, children.
fn chunk(id: &[u8; 4], content: &[u8], children: &[u8]) -> Vec<u8> {
    let mut out = id.to_vec();
    out.extend((content.len() as u32).to_le_bytes());
    out.extend((children.len() as u32).to_le_bytes());
    out.extend(content);
    out.extend(children);
    out
}

fn size(s: [u32; 3]) -> Vec<u8> {
    let mut c = Vec::new();
    for n in s {
        c.extend(n.to_le_bytes());
    }
    chunk(b"SIZE", &c, &[])
}

fn xyzi(voxels: &[[u8; 4]]) -> Vec<u8> {
    let mut c = (voxels.len() as u32).to_le_bytes().to_vec();
    for v in voxels {
        c.extend(v);
    }
    chunk(b"XYZI", &c, &[])
}

fn string(s: &str) -> Vec<u8> {
    let mut out = (s.len() as u32).to_le_bytes().to_vec();
    out.extend(s.as_bytes());
    out
}

fn dict(pairs: &[(&str, &str)]) -> Vec<u8> {
    let mut out = (pairs.len() as u32).to_le_bytes().to_vec();
    for (k, v) in pairs {
        out.extend(string(k));
        out.extend(string(v));
    }
    out
}

fn ntrn(id: u32, child: u32, frame: &[(&str, &str)]) -> Vec<u8> {
    let mut c = id.to_le_bytes().to_vec();
    c.extend(dict(&[]));
    c.extend(child.to_le_bytes());
    c.extend((-1i32).to_le_bytes());
    c.extend(0u32.to_le_bytes());
    c.extend(1u32.to_le_bytes());
    c.extend(dict(frame));
    chunk(b"nTRN", &c, &[])
}

fn ngrp(id: u32, children: &[u32]) -> Vec<u8> {
    let mut c = id.to_le_bytes().to_vec();
    c.extend(dict(&[]));
    c.extend((children.len() as u32).to_le_bytes());
    for child in children {
        c.extend(child.to_le_bytes());
    }
    chunk(b"nGRP", &c, &[])
}

fn nshp(id: u32, model: u32) -> Vec<u8> {
    let mut c = id.to_le_bytes().to_vec();
    c.extend(dict(&[]));
    c.extend(1u32.to_le_bytes());
    c.extend(model.to_le_bytes());
    c.extend(dict(&[]));
    chunk(b"nSHP", &c, &[])
}

fn file(children: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"VOX ".to_vec();
    out.extend(150u32.to_le_bytes());
    out.extend(chunk(b"MAIN", &[], &children.concat()));
    out
}

#[test]
fn one_voxel_is_six_sides() {
    let bytes = file(&[size([1, 1, 1]), xyzi(&[[0, 0, 0, 1]])]);
    let model = parse(&bytes).unwrap();
    assert_eq!(model.grid.size, [1, 1, 1]);
    assert_eq!(model.grid.voxels, vec![1]);
    assert_eq!(model.palette, default_palette());
    assert_eq!(model.palette[1], [0xff, 0xff, 0xff, 0xff]);
    let mesh = mesh(&model.grid, &model.palette);
    assert_eq!(mesh.vertices.len(), 24);
    assert_eq!(mesh.indices.len(), 36);
    // Every corner of a lone cube sees the whole sky, and stands on the
    // middle of its own footprint.
    assert!(mesh.vertices.iter().all(|v| v.sky == 255));
    for v in &mesh.vertices {
        assert!(v.position[0] == -0.5 || v.position[0] == 0.5);
        assert!(v.position[1] == -0.5 || v.position[1] == 0.5);
        assert!(v.position[2] == 0.0 || v.position[2] == 1.0);
    }
}

#[test]
fn the_grid_is_trimmed_to_its_voxels() {
    let bytes = file(&[size([8, 8, 8]), xyzi(&[[3, 4, 5, 7], [4, 4, 5, 9]])]);
    let model = parse(&bytes).unwrap();
    assert_eq!(model.grid.size, [2, 1, 1]);
    assert_eq!(model.grid.voxels, vec![7, 9]);
    // Two cubes side by side share a hidden side: ten show.
    assert_eq!(mesh(&model.grid, &model.palette).vertices.len(), 40);
}

#[test]
fn a_palette_in_the_file_is_read_off_by_one() {
    let mut rgba = Vec::new();
    for i in 0..=255u8 {
        rgba.extend([i, 0, 0, 255]);
    }
    let bytes = file(&[
        size([1, 1, 1]),
        xyzi(&[[0, 0, 0, 1]]),
        chunk(b"RGBA", &rgba, &[]),
    ]);
    let model = parse(&bytes).unwrap();
    // Colour 1 is the file's first entry.
    assert_eq!(model.palette[1], [0, 0, 0, 255]);
    assert_eq!(model.palette[255], [254, 0, 0, 255]);
    let mesh = mesh(&model.grid, &model.palette);
    assert_eq!(mesh.vertices[0].color, [0, 0, 0, 255]);
}

#[test]
fn two_models_placed_by_the_scene_graph() {
    // A 2 by 2 by 2 cube at the origin and a 1 by 1 by 1 one moved 3 along
    // x: MagicaVoxel places a model by its middle, the size halved rounded
    // down, so the cube's voxel (1, 1, 1) is at (0, 0, 0) and the lone voxel
    // is at (3, 0, 0).
    let bytes = file(&[
        size([2, 2, 2]),
        xyzi(&[
            [0, 0, 0, 1],
            [1, 0, 0, 1],
            [0, 1, 0, 1],
            [1, 1, 0, 1],
            [0, 0, 1, 1],
            [1, 0, 1, 1],
            [0, 1, 1, 1],
            [1, 1, 1, 1],
        ]),
        size([1, 1, 1]),
        xyzi(&[[0, 0, 0, 2]]),
        ntrn(0, 1, &[]),
        ngrp(1, &[2, 4]),
        ntrn(2, 3, &[("_t", "0 0 0")]),
        nshp(3, 0),
        ntrn(4, 5, &[("_t", "3 0 0")]),
        nshp(5, 1),
    ]);
    let model = parse(&bytes).unwrap();
    // From the cube's lowest voxel at -1 to the lone one at 3: five across.
    assert_eq!(model.grid.size, [5, 2, 2]);
    assert_eq!(model.grid.count(), 9);
    assert_eq!(model.grid.at([0, 0, 0]), 1);
    assert_eq!(model.grid.at([1, 1, 1]), 1);
    assert_eq!(model.grid.at([4, 1, 1]), 2);
    assert_eq!(model.grid.at([2, 0, 0]), 0);
}

#[test]
fn a_rotation_turns_a_model_about_its_middle() {
    // A row of three along x, turned a quarter about z: the byte 0b0100_01
    // maps output x from -y and output y from x. The row becomes a column
    // along y, where it stood.
    let row = [[0, 0, 0, 1], [1, 0, 0, 2], [2, 0, 0, 3]];
    let turned = file(&[
        size([3, 1, 1]),
        xyzi(&row),
        ntrn(0, 1, &[("_r", "17")]),
        nshp(1, 0),
    ]);
    let model = parse(&turned).unwrap();
    assert_eq!(model.grid.size, [1, 3, 1]);
    assert_eq!(model.grid.voxels, vec![1, 2, 3]);

    // Turned the other way, the colours run the other way.
    let back = file(&[
        size([3, 1, 1]),
        xyzi(&row),
        ntrn(0, 1, &[("_r", "33")]),
        nshp(1, 0),
    ]);
    let model = parse(&back).unwrap();
    assert_eq!(model.grid.size, [1, 3, 1]);
    assert_eq!(model.grid.voxels, vec![3, 2, 1]);
}

#[test]
fn what_is_refused() {
    assert_eq!(parse(b"VOX").err(), Some(Error::Truncated));
    assert_eq!(parse(b"GLTF1234").err(), Some(Error::NotVox));
    assert_eq!(parse(&file(&[])).err(), Some(Error::Empty));
    assert_eq!(
        parse(&file(&[size([1, 1, 1]), xyzi(&[])])).err(),
        Some(Error::Empty)
    );
    assert_eq!(
        parse(&file(&[size([SIDE + 1, 1, 1])])).err(),
        Some(Error::TooLarge)
    );
    let mut cut = file(&[size([1, 1, 1]), xyzi(&[[0, 0, 0, 1]])]);
    cut.truncate(cut.len() - 2);
    assert_eq!(parse(&cut).err(), Some(Error::Truncated));
    // A shape naming a model the file lacks.
    let dangling = file(&[
        size([1, 1, 1]),
        xyzi(&[[0, 0, 0, 1]]),
        ntrn(0, 1, &[]),
        nshp(1, 7),
    ]);
    assert_eq!(parse(&dangling).err(), Some(Error::Dangling));
    // A voxel outside its model's box is dropped, not kept.
    let outside = file(&[size([1, 1, 1]), xyzi(&[[0, 0, 0, 1], [5, 0, 0, 1]])]);
    assert_eq!(parse(&outside).unwrap().grid.size, [1, 1, 1]);
}

#[test]
fn hidden_corners_are_shut_in() {
    // An L of three voxels on the ground: the inner corner where the two arms
    // meet, on the top sides, sees less sky than an outer corner.
    let bytes = file(&[
        size([2, 2, 1]),
        xyzi(&[[0, 0, 0, 1], [1, 0, 0, 1], [0, 1, 0, 1]]),
    ]);
    let model = parse(&bytes).unwrap();
    let mesh = mesh(&model.grid, &model.palette);
    let tops: Vec<_> = mesh
        .vertices
        .iter()
        .filter(|v| v.normal == [0.0, 0.0, 1.0])
        .collect();
    assert_eq!(tops.len(), 12);
    // Only sides of a voxel at the inner corner lose sky to a neighbour.
    let sides: Vec<_> = mesh
        .vertices
        .iter()
        .filter(|v| v.normal[2] == 0.0)
        .collect();
    assert!(sides.iter().any(|v| v.sky < 255));
    assert!(tops.iter().all(|v| v.sky == 255));
}

#[test]
fn a_column_through_a_model_finds_its_solid_runs() {
    // A tower of three with air in the middle: voxels at z = 0 and z = 2.
    let bytes = file(&[size([1, 1, 3]), xyzi(&[[0, 0, 0, 1], [0, 0, 2, 1]])]);
    let grid = parse(&bytes).unwrap().grid;
    // Up the middle of the column, from under the model to over it: two
    // runs, each a voxel long, a third of the way each.
    let runs = grid.solid_runs([0.5, 0.5, -1.5], [0.5, 0.5, 4.5]);
    assert_eq!(runs.len(), 2, "{runs:?}");
    let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
    assert!(
        near(runs[0].0, 1.5 / 6.0) && near(runs[0].1, 2.5 / 6.0),
        "{runs:?}"
    );
    assert!(
        near(runs[1].0, 3.5 / 6.0) && near(runs[1].1, 4.5 / 6.0),
        "{runs:?}"
    );
    // Down the same column, the runs come in the other order.
    let down = grid.solid_runs([0.5, 0.5, 4.5], [0.5, 0.5, -1.5]);
    assert_eq!(down.len(), 2);
    assert!(near(down[0].0, 1.5 / 6.0), "{down:?}");
    // Beside the model: nothing.
    assert!(
        grid.solid_runs([1.5, 0.5, -1.0], [1.5, 0.5, 4.0])
            .is_empty()
    );
    // Adjacent solid voxels are one run: along a row of two.
    let row = parse(&file(&[
        size([2, 1, 1]),
        xyzi(&[[0, 0, 0, 1], [1, 0, 0, 1]]),
    ]))
    .unwrap()
    .grid;
    let runs = row.solid_runs([-1.0, 0.5, 0.5], [3.0, 0.5, 0.5]);
    assert_eq!(runs.len(), 1, "{runs:?}");
    assert!(near(runs[0].0, 0.25) && near(runs[0].1, 0.75), "{runs:?}");
    // A segment that ends inside solid ends its run at 1.
    let runs = row.solid_runs([-1.0, 0.5, 0.5], [1.0, 0.5, 0.5]);
    assert_eq!(runs.len(), 1);
    assert!(near(runs[0].0, 0.5) && near(runs[0].1, 1.0), "{runs:?}");
    // Slanting through a corner of the row, from below to above.
    let runs = row.solid_runs([0.2, 0.5, -0.5], [1.8, 0.5, 1.5]);
    assert_eq!(runs.len(), 1, "{runs:?}");
    assert!(near(runs[0].0, 0.25) && near(runs[0].1, 0.75), "{runs:?}");
}

/// The files MagicaVoxel wrote, from the reference checkouts beside the repo.
#[test]
fn files_magicavoxel_wrote() {
    let fixtures = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../refs/retro/test/fixtures"
    );
    let Ok(dir) = std::fs::read_dir(fixtures) else {
        return;
    };
    let mut seen = 0;
    for entry in dir.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "vox") {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        let model = parse(&bytes).unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
        assert!(model.grid.count() > 0, "{}", path.display());
        let mesh = mesh(&model.grid, &model.palette);
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.indices.len() % 3, 0);
        assert!(
            mesh.indices
                .iter()
                .all(|&i| (i as usize) < mesh.vertices.len())
        );
        seen += 1;
    }
    assert!(seen > 0);
}
