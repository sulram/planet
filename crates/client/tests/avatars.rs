//! The avatar flow over the asset seam, against the committed default set.

use client::{Client, Command, Event, Recipe};

fn serve(client: &mut Client) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    for request in client.drain_asset_requests() {
        let bytes = std::fs::read(root.join(&request.path)).map_err(|e| e.to_string());
        client.asset_loaded(request.id, bytes);
    }
}

fn worn(client: &mut Client) -> Vec<String> {
    let events = client.drain_events().into_iter();
    events
        .filter_map(|e| {
            if let Event::AvatarChanged { path } = e {
                Some(path)
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn next_avatar_walks_the_offer_and_wraps() {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    client.command(Command::SetAvatar {
        path: "avatars/Skull.vrm".into(),
    });
    serve(&mut client); // manifest and the avatar
    assert_eq!(worn(&mut client), ["avatars/Skull.vrm"]);

    // Skull is last in the manifest: next wraps to the first.
    client.command(Command::NextAvatar);
    serve(&mut client);
    assert_eq!(worn(&mut client), ["avatars/Bizdude.vrm"]);
}

#[test]
fn a_missing_avatar_falls_back_to_the_default() {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    serve(&mut client); // manifest
    client.command(Command::SetAvatar {
        path: "https://example.invalid/gone.vrm".into(),
    });
    serve(&mut client); // fails
    serve(&mut client); // the default
    assert_eq!(worn(&mut client), ["avatars/Kyle.vrm"]);
    let frame = client.settled_frame();
    assert_eq!((frame.skinned.len(), frame.boxes.len()), (1, 0));
}

#[test]
fn a_random_avatar_waits_for_the_manifest() {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    client.command(Command::RandomAvatar);
    serve(&mut client); // manifest, which then asks for the avatar
    serve(&mut client);
    assert_eq!(worn(&mut client).len(), 1);
}
