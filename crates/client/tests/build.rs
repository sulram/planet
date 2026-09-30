//! Building, as a front end asks for it: a tool in hand builds nothing, and
//! a platform is laid where it is asked for.

use client::{Base, BuildRefusal, Client, Command, Event, Input, Recipe, Tool};
use scene::VolumeChange;

/// A client standing on dry land, in the middle of a sector.
fn client() -> Client {
    let mut client = Client::new(Recipe::new(1)).expect("a world");
    client.go_to("4-77AEYRG").expect("a place to stand");
    client.update(1.0 / 60.0, &mut Input::default());
    client.drain_events();
    client.drain_volume_changes();
    client
}

fn refusals(client: &mut Client) -> Vec<BuildRefusal> {
    let refused = |event| match event {
        Event::BuildRefused { reason } => Some(reason),
        _ => None,
    };
    client
        .drain_events()
        .into_iter()
        .filter_map(refused)
        .collect()
}

#[test]
fn a_tool_in_hand_builds_nothing() {
    let mut client = client();
    client.command(Command::SetTool {
        tool: Some(Tool::Create),
    });
    client.update(1.0 / 60.0, &mut Input::default());
    assert!(client.building());
    assert!(refusals(&mut client).is_empty());
    let built = |change: &VolumeChange| matches!(change, VolumeChange::Add(..));
    assert!(!client.drain_volume_changes().iter().any(built));
}

#[test]
fn a_platform_is_laid_where_it_is_asked_for() {
    let mut client = client();
    client.command(Command::SetPlatform { side: 16 });
    client.command(Command::LayPlatform {
        base: Base::Pillars,
    });
    assert!(refusals(&mut client).is_empty());
    // It is laid and drawn over the next few frames, none of them at once.
    let built = |change: &VolumeChange| matches!(change, VolumeChange::Add(..));
    assert!(!client.drain_volume_changes().iter().any(built));
    let mut drawn = false;
    for _ in 0..10 {
        client.update(1.0 / 60.0, &mut Input::default());
        drawn |= client.drain_volume_changes().iter().any(built);
    }
    assert!(drawn);
    // Asking lays it with no tool in hand, and takes none.
    assert!(!client.building());
}
