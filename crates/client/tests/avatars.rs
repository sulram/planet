//! The avatar flow over the asset seam, against the committed default set.

use client::{Client, Command, Event, Recipe};

fn serve(client: &mut Client) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    for request in client.drain_asset_requests() {
        let bytes = std::fs::read(root.join(&request.path)).map_err(|e| e.to_string());
        client.asset_loaded(request.id, bytes);
    }
}

/// What the front end hears of the avatar: the choice, and whether it is worn.
fn heard(client: &mut Client) -> Vec<(Option<String>, bool)> {
    client
        .drain_events()
        .into_iter()
        .filter_map(|e| match e {
            Event::AvatarChanged { path, worn } => Some((path, worn)),
            _ => None,
        })
        .collect()
}

/// The avatars worn, in order.
fn worn(client: &mut Client) -> Vec<String> {
    heard(client)
        .into_iter()
        .filter_map(|(path, worn)| worn.then_some(path).flatten())
        .collect()
}

fn told(path: &str, worn: bool) -> (Option<String>, bool) {
    (Some(path.to_owned()), worn)
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
fn the_front_end_hears_the_wait_and_then_the_avatar() {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    client.command(Command::SetAvatar {
        path: "avatars/Skull.vrm".into(),
    });
    assert_eq!(heard(&mut client), [told("avatars/Skull.vrm", false)]);
    serve(&mut client);
    assert_eq!(heard(&mut client), [told("avatars/Skull.vrm", true)]);
}

/// The first press is one wait: it holds the whole offer, fetched one by
/// one in the order of pressing once the next has landed, so each press
/// after is worn at once with nothing left to fetch.
#[test]
fn the_first_press_fetches_the_offer_one_by_one_and_the_next_lands_at_once() {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    client.command(Command::SetAvatar {
        path: "avatars/Kyle.vrm".into(),
    });
    serve(&mut client); // manifest and Kyle
    serve(&mut client); // the clips
    heard(&mut client);

    client.command(Command::NextAvatar);
    assert_eq!(heard(&mut client), [told("avatars/MoonGirl.vrm", false)]);
    // The link goes whole to the one waited for; the rest follow one at a
    // time, each only after the one before it landed.
    let mut asked = Vec::new();
    loop {
        let requests = client.drain_asset_requests();
        if requests.is_empty() {
            break;
        }
        assert_eq!(requests.len(), 1);
        for request in requests {
            asked.push(request.path.clone());
            let bytes = std::fs::read(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../assets")
                    .join(&request.path),
            )
            .map_err(|e| e.to_string());
            client.asset_loaded(request.id, bytes);
        }
    }
    assert_eq!(
        asked,
        [
            "avatars/MoonGirl.vrm",
            "avatars/Mushy.vrm",
            "avatars/Olivia.vrm",
            "avatars/Shiro.vrm",
            "avatars/Skull.vrm",
            "avatars/Bizdude.vrm",
            "avatars/CoolAlien.vrm",
            "avatars/CoolBanana.vrm",
            "avatars/CoolBee.vrm",
        ]
    );
    assert_eq!(heard(&mut client), [told("avatars/MoonGirl.vrm", true)]);

    client.command(Command::NextAvatar);
    assert_eq!(heard(&mut client), [told("avatars/Mushy.vrm", true)]);
    assert!(client.drain_asset_requests().is_empty());
}

#[test]
fn a_missing_avatar_falls_back_to_the_default() {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    serve(&mut client); // manifest
    client.command(Command::SetAvatar {
        path: "https://example.invalid/gone.vrm".into(),
    });
    assert_eq!(
        heard(&mut client),
        [told("https://example.invalid/gone.vrm", false)]
    );
    serve(&mut client); // fails
    assert_eq!(heard(&mut client), [told("avatars/Kyle.vrm", false)]);
    serve(&mut client); // the default
    assert_eq!(heard(&mut client), [told("avatars/Kyle.vrm", true)]);
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

/// In a browser the avatar can land before any clip does.
#[test]
fn an_avatar_without_clips_stands_at_rest() {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    client.command(Command::SetAvatar {
        path: "avatars/Kyle.vrm".into(),
    });
    serve(&mut client); // manifest and the avatar; the clips stay unanswered
    client.drain_asset_requests();
    let frame = client.settled_frame();
    assert_eq!(frame.skinned.len(), 1);
    assert!(
        frame.skinned[0]
            .joints
            .iter()
            .all(|joint| joint.is_finite())
    );
}
