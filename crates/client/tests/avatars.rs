//! The avatar flow over the asset seam, against the committed default set.

use client::{Client, Command, Event, Outbound, Recipe};
use protocol::{Message, client_message, server_message};

/// An account's own avatar, on mundos's CDN: served here with the bytes of
/// a committed one.
const OWN: &str = "https://mundos-assets.tekne.studio/avatars/av1/0123abcd.vrm";

fn serve(client: &mut Client) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    for request in client.drain_asset_requests() {
        let path = if request.path == OWN {
            "avatars/Olivia.vrm"
        } else {
            &request.path
        };
        let bytes = std::fs::read(root.join(path)).map_err(|e| e.to_string());
        client.asset_loaded(request.id, bytes);
    }
}

/// A hosted world's welcome to an account that wears `avatar`, empty for
/// none, with `peers` already here.
fn welcome(client: &Client, avatar: &str, peers: Vec<protocol::Peer>) -> Vec<u8> {
    let recipe = client.recipe();
    protocol::encode(&protocol::ServerMessage {
        message: Some(server_message::Message::Welcome(protocol::Welcome {
            session: 1,
            recipe: Some(protocol::Recipe {
                seed: worldgen::format_seed(recipe.seed),
                generator_version: recipe.generator_version,
                params_json: serde_json::to_string(&recipe.params).unwrap(),
            }),
            peers,
            level: protocol::Level::SignedIn.into(),
            avatar: avatar.into(),
            ..Default::default()
        })),
    })
}

/// The avatars the client said it wears on the link, in Hello and Wear.
fn said(client: &mut Client) -> Vec<String> {
    client
        .drain_outbound()
        .into_iter()
        .filter_map(|out| match out {
            Outbound::Frame(frame) => {
                match protocol::ClientMessage::decode(&frame[..]).unwrap().message {
                    Some(client_message::Message::Hello(hello)) => Some(hello.avatar),
                    Some(client_message::Message::Wear(wear)) => Some(wear.avatar),
                    _ => None,
                }
            }
            Outbound::Close => None,
        })
        .collect()
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

/// A file that never came is forgotten: worn again, it is asked for again.
#[test]
fn an_avatar_that_never_came_is_asked_again() {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    serve(&mut client); // manifest
    let gone = "https://example.invalid/gone.vrm";
    client.command(Command::SetAvatar { path: gone.into() });
    serve(&mut client); // fails, and the default is asked for
    serve(&mut client); // the default and the clips
    client.command(Command::SetAvatar { path: gone.into() });
    let asked: Vec<String> = client
        .drain_asset_requests()
        .into_iter()
        .map(|request| request.path)
        .collect();
    assert_eq!(asked, [gone]);
}

/// The world's welcome dresses an account in its own avatar, whatever the
/// body wore before, and tells the others; `V` and a random pick pass.
#[test]
fn the_welcome_dresses_an_account_in_its_own_avatar() {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    client.command(Command::SetAvatar {
        path: "avatars/Kyle.vrm".into(),
    });
    serve(&mut client); // manifest and Kyle
    serve(&mut client); // the clips
    client.link_opened();
    assert_eq!(said(&mut client), ["avatars/Kyle.vrm"]);
    heard(&mut client);

    client.receive(&welcome(&client, OWN, vec![]));
    assert_eq!(said(&mut client), [OWN]);
    assert_eq!(heard(&mut client), [told(OWN, false)]);
    serve(&mut client);
    assert_eq!(heard(&mut client), [told(OWN, true)]);

    client.command(Command::NextAvatar);
    client.command(Command::RandomAvatar);
    assert!(heard(&mut client).is_empty());
    assert!(said(&mut client).is_empty());
    assert!(client.drain_asset_requests().is_empty());
}

/// A page that read the avatar on entering starts the body in it: the
/// welcome then changes nothing, and nothing more goes out.
#[test]
fn a_body_started_in_its_own_avatar_is_welcomed_as_it_is() {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    client.command(Command::SetAvatar { path: OWN.into() });
    serve(&mut client); // manifest and the avatar
    client.link_opened();
    assert_eq!(said(&mut client), [OWN]);
    heard(&mut client);

    client.receive(&welcome(&client, OWN, vec![]));
    assert!(said(&mut client).is_empty());
    assert!(heard(&mut client).is_empty());
}

/// With no avatar in the welcome, the choice stays the front end's.
#[test]
fn a_welcome_with_no_avatar_leaves_the_choice() {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    client.command(Command::SetAvatar {
        path: "avatars/Skull.vrm".into(),
    });
    serve(&mut client);
    client.link_opened();
    client.receive(&welcome(&client, "", vec![]));
    assert_eq!(said(&mut client), ["avatars/Skull.vrm"]);

    client.command(Command::NextAvatar);
    assert_eq!(said(&mut client), ["avatars/Bizdude.vrm"]);
}

/// A peer in an account's avatar is dressed from its URL, as any reference.
#[test]
fn a_peer_in_its_own_avatar_is_fetched_by_its_url() {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    serve(&mut client); // manifest
    client.link_opened();
    let peer = protocol::Peer {
        session: 2,
        name: "Ada".into(),
        avatar: OWN.into(),
        stance: Some(protocol::Stance {
            v: 30_000.0,
            u: 30_000.0,
            height_m: 5.0,
            facing_z: -1.0,
            ..Default::default()
        }),
        ..Default::default()
    };
    client.receive(&welcome(&client, "", vec![peer]));
    let asked: Vec<String> = client
        .drain_asset_requests()
        .iter()
        .map(|request| request.path.clone())
        .collect();
    assert!(asked.iter().any(|path| path == OWN), "{asked:?}");
}
