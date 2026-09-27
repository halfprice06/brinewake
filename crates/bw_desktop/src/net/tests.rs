//! Sessions end to end over real sockets on this machine.

use super::*;
use bw_core::{Faction, Kind, Pos};
use bw_sim::{Command, MapId, Outcome, World};
use std::time::{Duration, Instant};

fn worker_of(world: &World, player: u8) -> u32 {
    world
        .entities
        .iter()
        .filter(|e| e.owner == player && e.kind.is_worker())
        .map(|e| e.id)
        .min()
        .expect("a worker")
}

/// Step every session as fast as batches allow until each reaches `tick`.
fn run_to(sessions: &mut [&mut Session], tick: u64, within: Duration) {
    let deadline = Instant::now() + within;
    while sessions.iter().any(|s| s.tick() < tick) {
        assert!(
            Instant::now() < deadline,
            "sessions stuck at {:?}",
            ticks(sessions)
        );
        let mut moved = false;
        for session in sessions.iter_mut() {
            if session.tick() < tick {
                match session.try_step() {
                    StepOutcome::Stepped => moved = true,
                    StepOutcome::Stalled => {}
                    StepOutcome::Ended => panic!("ended early: {}", session.status()),
                }
            }
        }
        if !moved {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}

fn ticks(sessions: &[&mut Session]) -> Vec<u64> {
    sessions.iter().map(|s| s.tick()).collect()
}

/// Step like the game does: thirty ticks a second of wall time, a tick
/// that cannot step waits for the next frame. Returns the worst lag behind
/// the wall clock, in ticks, after the first second.
fn run_in_real_time(
    sessions: &mut [&mut Session],
    ticks: u64,
    mut each: impl FnMut(u64, &mut [&mut Session]),
) -> u64 {
    let started = Instant::now();
    let base: Vec<u64> = sessions.iter().map(|s| s.tick()).collect();
    let mut worst = 0;
    loop {
        let due = (started.elapsed().as_secs_f64() * 30.0) as u64;
        if due > ticks + 30 {
            panic!(
                "could not keep up: {:?}",
                sessions.iter().map(|s| s.tick()).collect::<Vec<_>>()
            );
        }
        for (i, session) in sessions.iter_mut().enumerate() {
            while session.tick() - base[i] < due.min(ticks) {
                match session.try_step() {
                    StepOutcome::Stepped => {}
                    StepOutcome::Stalled => break,
                    StepOutcome::Ended => panic!("ended early: {}", session.status()),
                }
            }
            if due > 30 && due <= ticks {
                worst = worst.max(due - (session.tick() - base[i]));
            }
        }
        let low = sessions.iter().map(|s| s.tick()).min().unwrap();
        each(low, sessions);
        if sessions
            .iter()
            .enumerate()
            .all(|(i, s)| s.tick() - base[i] >= ticks)
        {
            return worst;
        }
        std::thread::sleep(Duration::from_millis(4));
    }
}

#[test]
fn two_peers_stay_in_lockstep_with_commands_from_both_seats() {
    let (mut host, mut guest) = local_pair(77, Faction::Union, 3);
    assert_eq!(host.local, 0);
    assert_eq!(guest.local, 1);
    assert_eq!(guest.local_faction(), Faction::Assembly);
    let host_worker = host
        .canonical()
        .entities
        .iter()
        .find(|e| e.owner == 0 && e.kind == Kind::Hook)
        .map(|e| e.id)
        .expect("host worker");
    let guest_worker = worker_of(&guest.view(), 0);
    assert_ne!(host_worker, guest_worker);
    let start_of = |world: &World, id: u32| world.entities.iter().find(|e| e.id == id).unwrap().pos;
    let host_start = start_of(host.canonical(), host_worker);
    let guest_start = start_of(host.canonical(), guest_worker);
    run_to(&mut [&mut host, &mut guest], 5, Duration::from_secs(10));
    host.issue(Command::Move {
        units: vec![host_worker],
        target: Pos::cell(30, 64),
        queued: false,
    })
    .expect("host move");
    run_to(&mut [&mut host, &mut guest], 9, Duration::from_secs(10));
    guest
        .issue(Command::Move {
            units: vec![guest_worker],
            target: Pos::cell(100, 64),
            queued: false,
        })
        .expect("guest move");
    run_to(&mut [&mut host, &mut guest], 240, Duration::from_secs(20));
    assert_eq!(host.canonical(), guest.canonical());
    assert!(host.desync.is_none() && guest.desync.is_none());
    for world in [host.canonical(), guest.canonical()] {
        let a = world.entities.iter().find(|e| e.id == host_worker).unwrap();
        let b = world
            .entities
            .iter()
            .find(|e| e.id == guest_worker)
            .unwrap();
        assert!(world.command_log.iter().filter(|r| r.accepted).count() >= 2);
        assert_ne!(a.pos, host_start);
        assert_ne!(b.pos, guest_start);
    }
    guest.close();
    host.close();
}

#[test]
fn a_bad_connection_keeps_the_match_in_step_and_close_to_real_time() {
    // 60 ms each way, up to 25 ms more, one packet in ten lost, both ways.
    let bad = Conditions {
        latency_ms: 60,
        jitter_ms: 25,
        loss_percent: 10,
    };
    let options = LobbyOptions {
        conditions: bad,
        auto: true,
        ..LobbyOptions::local()
    };
    let mut host_lobby =
        Lobby::host("127.0.0.1:0", Faction::Assembly, 31, options.clone()).unwrap();
    let addr = host_lobby.local_addr().unwrap();
    let mut guest_lobby =
        Lobby::join(&JoinCode::parse(&addr.to_string()).unwrap(), None, options).unwrap();
    let (mut host, mut guest) = (None, None);
    // Lost packets and a busy machine (the whole suite runs at once) can slow
    // the handshake; the lobby itself waits two minutes.
    let deadline = Instant::now() + Duration::from_secs(60);
    while (host.is_none() || guest.is_none()) && Instant::now() < deadline {
        host = host.or_else(|| host_lobby.poll());
        guest = guest.or_else(|| guest_lobby.poll());
        std::thread::sleep(Duration::from_millis(2));
    }
    let (mut host, mut guest) = (host.expect("host"), guest.expect("guest"));
    // The real-time clock below starts now, not at the agreed moment.
    host.start_now();
    guest.start_now();
    let host_worker = worker_of(host.canonical(), 0);
    let guest_worker = worker_of(guest.canonical(), 1);
    let mut orders = 0;
    let worst = run_in_real_time(&mut [&mut host, &mut guest], 300, |tick, sessions| {
        // An order from each side every second.
        if tick >= orders * 30 + 15 {
            orders += 1;
            let x = if orders % 2 == 0 { 40 } else { 50 };
            let _ = sessions[0].issue(Command::Move {
                units: vec![host_worker],
                target: Pos::cell(x, 60),
                queued: false,
            });
            let _ = sessions[1].issue(Command::Move {
                units: vec![guest_worker],
                target: Pos::cell(x + 40, 60),
                queued: false,
            });
        }
    });
    let level = host.tick().max(guest.tick());
    run_to(&mut [&mut host, &mut guest], level, Duration::from_secs(5));
    assert_eq!(host.canonical(), guest.canonical(), "the worlds agree");
    assert!(host.desync.is_none() && guest.desync.is_none());
    let accepted = host
        .canonical()
        .command_log
        .iter()
        .filter(|r| r.accepted)
        .count();
    assert!(accepted >= 12, "orders from both seats landed: {accepted}");
    // The delay grew to cover the path: 60-85 ms each way is 3 or more
    // ticks with the margin.
    assert!(
        host.delay >= 3 && guest.delay >= 3,
        "delays {} {}",
        host.delay,
        guest.delay
    );
    assert!(
        host.delay <= 7 && guest.delay <= 7,
        "delays {} {}",
        host.delay,
        guest.delay
    );
    // Neither seat fell far behind the wall clock once running.
    eprintln!(
        "worst lag {worst} ticks, delays {} {}",
        host.delay, guest.delay
    );
    assert!(worst <= 8, "worst lag {worst} ticks");
    let rtt = host.net_view().rtt_ms.expect("measured");
    assert!((100..300).contains(&rtt), "round trip {rtt}");
}

#[test]
fn a_third_seat_watches_the_same_match_through_the_host() {
    let options = LobbyOptions {
        fixed_delay: Some(3),
        ..LobbyOptions::local()
    };
    let mut host = Lobby::host("127.0.0.1:0", Faction::Union, 5, options.clone()).unwrap();
    let code = JoinCode::parse(&host.local_addr().unwrap().to_string()).unwrap();
    let mut a = Lobby::join(
        &code,
        None,
        LobbyOptions {
            name: "ANNE".into(),
            ..options.clone()
        },
    )
    .unwrap();
    let mut b = Lobby::join(
        &code,
        None,
        LobbyOptions {
            name: "BO".into(),
            ..options
        },
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while (host.seats.len() < 3 || a.seats.len() < 3 || b.seats.len() < 3)
        && Instant::now() < deadline
    {
        host.poll();
        a.poll();
        b.poll();
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(host.seats.len(), 3);
    // Whoever joined second watches: the map holds two.
    let watcher_is_a = a.seats[a.you.unwrap() as usize].faction.is_none();
    let (player, watcher) = if watcher_is_a {
        (&mut b, &mut a)
    } else {
        (&mut a, &mut b)
    };
    assert!(
        watcher.seats[watcher.you.unwrap() as usize]
            .faction
            .is_none()
    );
    assert!(
        host.can_start().is_err(),
        "the guest player is not ready yet"
    );
    player.set_ready(true);
    let deadline = Instant::now() + Duration::from_secs(5);
    while host.can_start().is_err() && Instant::now() < deadline {
        host.poll();
        player.poll();
        watcher.poll();
        std::thread::sleep(Duration::from_millis(2));
    }
    let mut h = host
        .start()
        .expect("starts without the watcher being ready");
    let (mut p, mut w) = (None, None);
    let deadline = Instant::now() + Duration::from_secs(5);
    while (p.is_none() || w.is_none()) && Instant::now() < deadline {
        p = p.or_else(|| player.poll());
        w = w.or_else(|| watcher.poll());
        std::thread::sleep(Duration::from_millis(2));
    }
    let (mut p, mut w) = (p.unwrap(), w.unwrap());
    for s in [&mut h, &mut p, &mut w] {
        s.start_now();
    }
    assert!(w.watching());
    assert!(w.view().revealed, "a watcher sees the whole field");
    assert!(w.issue(Command::Stop { units: vec![] }).is_err());
    let worker = worker_of(p.canonical(), 1);
    run_to(&mut [&mut h, &mut p, &mut w], 20, Duration::from_secs(10));
    p.issue(Command::Move {
        units: vec![worker],
        target: Pos::cell(90, 50),
        queued: false,
    })
    .unwrap();
    run_to(&mut [&mut h, &mut p, &mut w], 200, Duration::from_secs(20));
    assert_eq!(h.canonical(), p.canonical());
    assert_eq!(
        h.canonical(),
        w.canonical(),
        "the watcher's world is the same"
    );
    assert!(h.desync.is_none() && p.desync.is_none() && w.desync.is_none());
    // The watcher leaving changes nothing for the players.
    w.close();
    drop(w);
    run_to(&mut [&mut h, &mut p], 260, Duration::from_secs(10));
    assert_eq!(h.canonical(), p.canonical());
    assert!(h.canonical().outcome.is_none());
}

#[test]
fn three_players_play_the_confluence_and_one_leaving_leaves_two() {
    let options = LobbyOptions {
        fixed_delay: Some(3),
        ..LobbyOptions::local()
    };
    let mut host = Lobby::host("127.0.0.1:0", Faction::Union, 6, options.clone()).unwrap();
    host.set_map(MapId::Confluence);
    let code = JoinCode::parse(&host.local_addr().unwrap().to_string()).unwrap();
    let mut guests: Vec<Lobby> = ["ANNE", "BO"]
        .into_iter()
        .map(|name| {
            Lobby::join(
                &code,
                None,
                LobbyOptions {
                    name: name.into(),
                    auto: true,
                    ..options.clone()
                },
            )
            .unwrap()
        })
        .collect();
    let deadline = Instant::now() + Duration::from_secs(10);
    while host.can_start().is_err() && Instant::now() < deadline {
        host.poll();
        for guest in &mut guests {
            guest.poll();
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(guests.iter().all(|g| g.map == MapId::Confluence));
    assert!(host.seats.iter().all(|s| s.faction.is_some()), "three play");
    let mut h = host.start().expect("three ready players start");
    let mut sessions = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(5);
    while sessions.len() < 2 && Instant::now() < deadline {
        for guest in &mut guests {
            if let Some(session) = guest.poll() {
                sessions.push(session);
            }
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    let (mut b, mut c) = (sessions.remove(0), sessions.remove(0));
    for s in [&mut h, &mut b, &mut c] {
        s.start_now();
    }
    assert_eq!(h.canonical().seat_count(), 3);
    assert_eq!(h.canonical().map.id, MapId::Confluence);
    assert!(!b.watching() && !c.watching());
    run_to(&mut [&mut h, &mut b, &mut c], 90, Duration::from_secs(20));
    assert_eq!(h.canonical(), b.canonical());
    assert_eq!(h.canonical(), c.canonical());
    // One guest leaves: it surrenders, and the other two play on.
    let gone = c.local;
    c.close();
    drop(c);
    let deadline = Instant::now() + Duration::from_secs(10);
    while !h.canonical().is_eliminated(gone) {
        assert!(Instant::now() < deadline, "host at {}", h.tick());
        let _ = h.try_step();
        let _ = b.try_step();
        std::thread::sleep(Duration::from_millis(1));
    }
    let tick = h.tick() + 60;
    run_to(&mut [&mut h, &mut b], tick, Duration::from_secs(10));
    assert_eq!(h.canonical(), b.canonical());
    assert!(h.canonical().outcome.is_none(), "two are still playing");
    assert!(h.desync.is_none() && b.desync.is_none());
}

/// Three connected seats on the Confluence: host, then two guests.
pub(crate) fn confluence_three(seed: u64) -> (Session, Session, Session) {
    let (h, b, c, _) = confluence_three_named(seed, ["ANNE", "BO"]);
    (h, b, c)
}

/// Three connected seats on the Confluence, the guests going by `names`,
/// and the host's code for joining again.
fn confluence_three_named(seed: u64, names: [&str; 2]) -> (Session, Session, Session, JoinCode) {
    let options = LobbyOptions {
        fixed_delay: Some(3),
        ..LobbyOptions::local()
    };
    let mut host = Lobby::host("127.0.0.1:0", Faction::Union, seed, options.clone()).unwrap();
    host.set_map(MapId::Confluence);
    let code = JoinCode::parse(&host.local_addr().unwrap().to_string()).unwrap();
    let mut guests: Vec<Lobby> = names
        .into_iter()
        .map(|name| {
            let options = LobbyOptions {
                name: name.into(),
                auto: true,
                ..options.clone()
            };
            let lobby = Lobby::join(&code, None, options).unwrap();
            // One at a time, so the seats go out in joining order.
            std::thread::sleep(Duration::from_millis(50));
            lobby
        })
        .collect();
    let deadline = Instant::now() + Duration::from_secs(10);
    while host.can_start().is_err() && Instant::now() < deadline {
        host.poll();
        for guest in &mut guests {
            guest.poll();
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    let mut h = host.start().expect("three ready players start");
    let mut sessions = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(5);
    while sessions.len() < 2 && Instant::now() < deadline {
        for guest in &mut guests {
            if let Some(session) = guest.poll() {
                sessions.push(session);
            }
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    let (mut b, mut c) = (sessions.remove(0), sessions.remove(0));
    if b.local_seat > c.local_seat {
        std::mem::swap(&mut b, &mut c);
    }
    for s in [&mut h, &mut b, &mut c] {
        s.start_now();
    }
    (h, b, c, code)
}

/// Keep `sessions` stepping for `span` while every other seat's game is
/// busy and steps nothing, as in one long tick.
fn step_for(sessions: &mut [&mut Session], span: Duration) {
    let until = Instant::now() + span;
    while Instant::now() < until {
        for session in sessions.iter_mut() {
            let _ = session.try_step();
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn a_busy_seat_is_waited_for_and_never_dropped_but_a_gone_one_is() {
    // Trial 10: the simulation crawled at a few seconds a tick, and the
    // seats showed a drop clock counting down on a seat that was only busy.
    let (mut h, mut b, mut c) = confluence_three(31);
    assert_eq!((h.local_seat, b.local_seat, c.local_seat), (0, 1, 2));
    run_to(&mut [&mut h, &mut b, &mut c], 60, Duration::from_secs(20));
    let drop_after = Duration::from_secs(4);
    for s in [&mut h, &mut b, &mut c] {
        s.set_drop_after(drop_after);
    }
    // The third seat's game is busy (a tick far longer than the drop
    // time) and its game thread touches nothing. Its link keeps the
    // connection alive, so nobody drops it and no drop clock is shown.
    step_for(&mut [&mut h, &mut b], Duration::from_millis(5000));
    assert!(!h.canonical().is_eliminated(c.local), "a busy seat is kept");
    for (who, s) in [("host", &h), ("guest", &b)] {
        let view = s.net_view();
        assert_eq!(view.waiting, None, "{who}: no drop clock for a busy seat");
        let (seat, name, late) = view.slow.clone().expect("the busy seat is named");
        assert_eq!(seat, 2, "{who}");
        assert_eq!(name, s.seat_name(2), "{who}");
        assert!(late >= 2, "{who}: {view:?}");
        assert!(!s.waiting_on_peer() && s.waiting_on_slow_peer(), "{who}");
        assert!(s.status().contains("IS BEHIND"), "{who}: {}", s.status());
    }
    // The host's game is busy in turn: the guests keep it too.
    step_for(&mut [&mut b, &mut c], Duration::from_millis(4500));
    assert!(b.ending().is_none() && c.ending().is_none());
    // Everyone catches up and plays on in step, nobody dropped.
    let tick = h.tick().max(b.tick()).max(c.tick()) + 60;
    run_to(&mut [&mut h, &mut b, &mut c], tick, Duration::from_secs(20));
    assert_eq!(h.canonical(), b.canonical());
    assert_eq!(h.canonical(), c.canonical());
    assert!((0..3).all(|p| !h.canonical().is_eliminated(p)));
    assert!(h.net_view().slow.is_none() && h.net_view().waiting.is_none());
    // Now the third seat is truly gone: its connection falls silent. The
    // guest sees the host's drop clock run for it, and the host drops it.
    let gone = c.local;
    c.crash();
    let started = Instant::now();
    let mut clock_seen = false;
    while !h.canonical().is_eliminated(gone) {
        assert!(
            started.elapsed() < Duration::from_secs(15),
            "host at {}: {}",
            h.tick(),
            h.status()
        );
        let _ = h.try_step();
        let _ = b.try_step();
        if let Some((seat, _, _, left)) = b.net_view().waiting {
            assert_eq!(seat, 2);
            assert!(left <= 2, "the guest shows the host's drop clock");
            clock_seen = true;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        // Silence counts from the last keepalive, up to 200 ms before.
        started.elapsed() + Duration::from_millis(300) >= drop_after,
        "dropped only after the grace: {:?}",
        started.elapsed()
    );
    assert!(
        clock_seen,
        "the guest saw the drop clock for the silent seat"
    );
    let tick = h.tick() + 60;
    run_to(&mut [&mut h, &mut b], tick, Duration::from_secs(10));
    assert_eq!(h.canonical(), b.canonical());
    assert!(h.canonical().outcome.is_none(), "two are still playing");
}

#[test]
fn a_player_who_says_goodbye_surrenders_and_the_other_wins() {
    let (mut host, mut guest) = local_pair(3, Faction::Union, 3);
    run_to(&mut [&mut host, &mut guest], 60, Duration::from_secs(10));
    guest.close();
    drop(guest);
    let deadline = Instant::now() + Duration::from_secs(10);
    while host.canonical().outcome.is_none() {
        assert!(
            Instant::now() < deadline,
            "host at {}: {}",
            host.tick(),
            host.status()
        );
        if host.try_step() == StepOutcome::Stalled {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    assert_eq!(host.canonical().outcome, Some(Outcome::Victory(0)));
    assert!(host.take_notices().iter().any(|n| n.ends_with("LEFT")));
}

#[test]
fn a_guest_whose_host_leaves_wins_a_two_player_match() {
    let (mut host, mut guest) = local_pair(4, Faction::Assembly, 3);
    run_to(&mut [&mut host, &mut guest], 45, Duration::from_secs(10));
    host.close();
    drop(host);
    let deadline = Instant::now() + Duration::from_secs(10);
    while guest.canonical().outcome.is_none() {
        assert!(
            Instant::now() < deadline,
            "guest at {}: {}",
            guest.tick(),
            guest.status()
        );
        if guest.try_step() == StepOutcome::Stalled {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    // The guest is player 1 in the canonical world, player 0 in its view.
    assert_eq!(guest.canonical().outcome, Some(Outcome::Victory(1)));
    assert_eq!(guest.view().outcome, Some(Outcome::Victory(0)));
}

#[test]
fn a_silent_peer_is_waited_for_and_named_then_the_match_goes_on() {
    let (mut host, mut guest) = local_pair(6, Faction::Union, 3);
    run_to(&mut [&mut host, &mut guest], 30, Duration::from_secs(10));
    assert!(!host.waiting_on_peer());
    assert_eq!(host.status(), "");
    // The guest's connection goes dead for a few seconds.
    guest.set_conditions(Conditions {
        loss_percent: 100,
        ..Default::default()
    });
    let outage = Instant::now();
    while outage.elapsed() < Duration::from_millis(2600) {
        let _ = host.try_step();
        let _ = guest.try_step();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(host.waiting_on_peer(), "{}", host.status());
    let view = host.net_view();
    let (seat, name, quiet, left) = view.waiting.clone().expect("waiting");
    assert_eq!(seat, 1);
    assert_eq!(name, guest.seat_name(1));
    assert!(quiet >= 2 && left <= 58, "{view:?}");
    assert!(host.status().starts_with("WAITING FOR "));
    // It comes back: nothing was lost, the match carries on in step.
    guest.set_conditions(Conditions::default());
    let resume = host.tick() + 60;
    run_to(
        &mut [&mut host, &mut guest],
        resume,
        Duration::from_secs(10),
    );
    assert_eq!(host.canonical(), guest.canonical());
    assert!(!host.waiting_on_peer());
}

#[test]
fn a_crashed_guest_rejoins_with_its_token_and_the_match_goes_on_in_step() {
    let options = LobbyOptions {
        fixed_delay: Some(3),
        auto: true,
        ..LobbyOptions::local()
    };
    let mut host_lobby = Lobby::host("127.0.0.1:0", Faction::Union, 12, options.clone()).unwrap();
    let code = JoinCode::parse(&host_lobby.local_addr().unwrap().to_string()).unwrap();
    let mut guest_lobby = Lobby::join(&code, None, options.clone()).unwrap();
    let (mut host, mut guest) = (None, None);
    let deadline = Instant::now() + Duration::from_secs(10);
    while (host.is_none() || guest.is_none()) && Instant::now() < deadline {
        host = host.or_else(|| host_lobby.poll());
        guest = guest.or_else(|| guest_lobby.poll());
        std::thread::sleep(Duration::from_millis(1));
    }
    let token = guest_lobby.token.expect("a rejoin token");
    let (mut host, mut guest) = (host.unwrap(), guest.unwrap());
    host.start_now();
    guest.start_now();
    let guest_worker = worker_of(guest.canonical(), 1);
    run_to(&mut [&mut host, &mut guest], 40, Duration::from_secs(10));
    guest
        .issue(Command::Move {
            units: vec![guest_worker],
            target: Pos::cell(95, 70),
            queued: false,
        })
        .unwrap();
    run_to(&mut [&mut host, &mut guest], 90, Duration::from_secs(10));
    guest.crash();
    // The host waits for the missing seat.
    let stall = Instant::now();
    while stall.elapsed() < Duration::from_millis(300) {
        let _ = host.try_step();
        std::thread::sleep(Duration::from_millis(2));
    }
    let held = host.tick();
    assert!(
        held <= 94,
        "the host cannot run on without the guest: {held}"
    );
    // A new game on the guest's machine joins with the same code and token.
    let mut again = Lobby::join(&code, Some(token), options).unwrap();
    let mut back = None;
    let deadline = Instant::now() + Duration::from_secs(10);
    while back.is_none() && Instant::now() < deadline {
        let _ = host.try_step();
        back = again.poll();
        std::thread::sleep(Duration::from_millis(1));
    }
    let mut back = back.expect("rejoined");
    assert_eq!(back.local_seat, 1);
    assert_eq!(back.tick(), held, "it picks up where the host stands");
    run_to(
        &mut [&mut host, &mut back],
        held + 150,
        Duration::from_secs(20),
    );
    assert_eq!(host.canonical(), back.canonical());
    assert!(host.desync.is_none() && back.desync.is_none());
    assert!(host.take_notices().iter().any(|n| n.ends_with("IS BACK")));
    // And it can still give orders.
    back.issue(Command::Stop {
        units: vec![guest_worker],
    })
    .unwrap();
    let target = back.tick() + 30;
    run_to(&mut [&mut host, &mut back], target, Duration::from_secs(10));
    assert_eq!(host.canonical(), back.canonical());
}

#[test]
fn a_desync_stops_the_match_on_both_seats_and_says_so() {
    let (mut host, mut guest) = local_pair(8, Faction::Union, 3);
    run_to(&mut [&mut host, &mut guest], 20, Duration::from_secs(10));
    guest.canonical_mut().players[0].salvage += 1;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let a = host.try_step();
        let b = guest.try_step();
        if a == StepOutcome::Ended && b == StepOutcome::Ended {
            break;
        }
        assert!(Instant::now() < deadline, "no desync seen");
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(host.desync, Some(30));
    assert_eq!(guest.desync, Some(30));
    assert_eq!(
        host.ending(),
        Some(&Ending::Desync {
            tick: 30,
            seats: vec![1]
        })
    );
    assert_eq!(host.status(), "DESYNC AT TICK 30");
    let history = host.hash_history();
    let (tick, mine, theirs) = history.last().unwrap();
    assert_eq!(*tick, 30);
    assert_ne!(Some(mine), theirs.as_ref());
    assert!(host.issue(Command::Stop { units: vec![] }).is_err());
}

#[test]
fn a_guest_with_other_rules_is_refused_with_a_reason() {
    assert!(
        session::check_hello(
            protocol::PROTOCOL,
            &bw_content::rules_digest(),
            bw_content::build_fingerprint()
        )
        .is_ok()
    );
    let old = session::check_hello(2, &bw_content::rules_digest(), "x").unwrap_err();
    assert!(old.contains("version"), "{old}");
    let rules = session::check_hello(protocol::PROTOCOL, "other", bw_content::build_fingerprint())
        .unwrap_err();
    assert!(rules.contains("different rules"), "{rules}");
    assert!(
        session::check_build("0123456789abcdef")
            .unwrap_err()
            .contains("different build")
    );
    assert!(session::check_build("unknown").is_ok());
}

#[test]
fn a_host_gives_up_when_no_guest_arrives() {
    let started = Instant::now();
    let result = lobby::host_blocking(
        "127.0.0.1:0",
        1,
        Faction::Union,
        bw_sim::MapId::SplitBasin,
        Some(3),
        Duration::from_millis(300),
    );
    assert!(
        result
            .err()
            .expect("no session")
            .contains("no other player")
    );
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "it does not hang"
    );
}

#[test]
fn a_guest_without_a_host_is_told_so() {
    let dead = std::net::UdpSocket::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    let result = lobby::join_blocking(&dead.to_string(), Duration::from_millis(500), None);
    assert!(result.is_err());
}

#[test]
fn a_world_sent_to_a_rejoining_guest_steps_exactly_as_the_original() {
    // The rejoin rests on this: a world serialised mid-match and read back
    // steps identically to the one that stayed in memory.
    let mut world = World::new(21, Faction::Union);
    world.ai_enabled = false;
    let worker = worker_of(&world, 0);
    for _ in 0..50 {
        world.step();
    }
    world
        .issue(
            0,
            Command::Move {
                units: vec![worker],
                target: Pos::cell(40, 40),
                queued: false,
            },
        )
        .unwrap();
    for _ in 0..10 {
        world.step();
    }
    let bytes = protocol::Wire::Resume {
        seat: 1,
        plan: protocol::MatchPlan {
            seed: 21,
            seats: vec![],
            map: MapId::SplitBasin,
        },
        world: Box::new(world.clone()),
        batches: vec![],
        upto: vec![],
        dropped: vec![],
        delay: 3,
    }
    .encode();
    let protocol::Wire::Resume { world: copy, .. } = protocol::Wire::decode(&bytes).unwrap() else {
        panic!("round trip");
    };
    let mut copy = *copy;
    assert_eq!(copy.state_hash(), world.state_hash());
    for _ in 0..600 {
        world.step();
        copy.step();
    }
    assert_eq!(copy.state_hash(), world.state_hash());
    assert!(
        bytes.len() < 4_000_000,
        "a snapshot is {} bytes",
        bytes.len()
    );
}

/// Step `sessions` until `until` holds, failing after `within`.
fn step_until(
    sessions: &mut [&mut Session],
    within: Duration,
    what: &str,
    mut until: impl FnMut(&mut [&mut Session]) -> bool,
) {
    let deadline = Instant::now() + within;
    while !until(sessions) {
        assert!(
            Instant::now() < deadline,
            "{what}: ticks {:?}, {}",
            ticks(sessions),
            sessions[0].status()
        );
        for session in sessions.iter_mut() {
            let _ = session.try_step();
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Every machine of `player` holds where it stands.
fn all_hold(world: &World, player: u8) -> bool {
    let mut machines = world
        .entities
        .iter()
        .filter(|e| e.owner == player && e.hp > 0 && !e.kind.is_building())
        .peekable();
    machines.peek().is_some() && machines.all(|e| e.order == bw_sim::Order::Hold)
}

#[test]
fn a_crashed_guest_of_three_is_kept_open_while_the_others_play_on_then_rejoins_in_step() {
    // Two guests who both go by ANNE: the second is "ANNE 2" in the lobby,
    // and comes back by its own name without a token.
    let (mut h, mut b, mut c, code) = confluence_three_named(52, ["ANNE", "ANNE"]);
    assert_eq!((h.local_seat, b.local_seat, c.local_seat), (0, 1, 2));
    assert_eq!(h.seat_name(2), "ANNE 2");
    let gone = c.local;
    let worker = worker_of(c.canonical(), gone);
    run_to(&mut [&mut h, &mut b, &mut c], 30, Duration::from_secs(20));
    c.issue(Command::Move {
        units: vec![worker],
        target: Pos::cell(64, 40),
        queued: false,
    })
    .unwrap();
    run_to(&mut [&mut h, &mut b, &mut c], 90, Duration::from_secs(20));
    assert_eq!(h.canonical(), c.canonical());
    // The third seat's game crashes. The others wait, and the host may keep
    // its seat open; a guest may not.
    c.crash();
    step_until(
        &mut [&mut h, &mut b],
        Duration::from_secs(10),
        "the host is offered to keep the seat",
        |s| s[0].net_view().can_keep_open,
    );
    let waited = h.tick();
    assert_eq!(h.net_view().waiting.as_ref().map(|w| w.0), Some(2));
    assert!(!b.net_view().can_keep_open, "only the host decides");
    assert!(b.keep_seat_open(2).is_err());
    assert!(h.keep_seat_open(0).is_err(), "not the host's own seat");
    h.keep_seat_open(2).expect("the host keeps the seat open");
    assert!(!h.can_keep_open(2), "once is enough");
    // The other two play on: five seconds of match with the seat empty.
    let on = waited + 150;
    run_to(&mut [&mut h, &mut b], on, Duration::from_secs(20));
    assert_eq!(h.canonical(), b.canonical());
    for world in [h.canonical(), b.canonical()] {
        assert!(!world.is_eliminated(gone), "a seat kept open is still in");
        assert!(
            !world
                .command_log
                .iter()
                .any(|r| r.player == gone && matches!(r.command, Command::Surrender)),
            "and it has not surrendered"
        );
        assert!(all_hold(world, gone), "its machines hold where they stand");
    }
    let (away, name, left) = b
        .net_view()
        .away
        .expect("the guest sees the seat kept open");
    assert_eq!((away, name.as_str()), (2, "ANNE 2"));
    assert!(left > 200 && left <= 300, "{left}");
    assert!(h.status().contains("AWAY"), "{}", h.status());
    // Its player starts the game again and joins with the host's code and
    // its name alone: it is sent the match as it stands and takes its seat.
    let options = LobbyOptions {
        name: "ANNE".into(),
        fixed_delay: Some(3),
        ..LobbyOptions::local()
    };
    let mut again = Lobby::join(&code, None, options).unwrap();
    let mut back = None;
    step_until(
        &mut [&mut h, &mut b],
        Duration::from_secs(10),
        "the guest is sent the match",
        |_| {
            back = back.take().or_else(|| again.poll());
            back.is_some()
        },
    );
    let mut back = back.unwrap();
    assert_eq!(back.local_seat, 2);
    assert_eq!(back.local, gone);
    assert!(back.tick() >= on);
    // All three play on in step; the returned seat gives orders again.
    let tick = h.tick().max(b.tick()).max(back.tick()) + 60;
    run_to(
        &mut [&mut h, &mut b, &mut back],
        tick,
        Duration::from_secs(20),
    );
    back.issue(Command::Move {
        units: vec![worker],
        target: Pos::cell(60, 44),
        queued: false,
    })
    .unwrap();
    let tick = tick + 150;
    run_to(
        &mut [&mut h, &mut b, &mut back],
        tick,
        Duration::from_secs(20),
    );
    assert_eq!(h.canonical(), b.canonical());
    assert_eq!(h.canonical(), back.canonical());
    let hash = h.canonical().state_hash();
    assert_eq!(b.canonical().state_hash(), hash);
    assert_eq!(back.canonical().state_hash(), hash);
    assert!(h.desync.is_none() && b.desync.is_none() && back.desync.is_none());
    // The hashes exchanged since agree too, on every seat.
    for s in [&h, &b, &back] {
        let history = s.hash_history();
        assert!(history.iter().any(|(t, _, _)| *t > on), "hashes since");
        for (tick, mine, other) in history {
            if let Some(other) = other {
                assert_eq!(mine, other, "tick {tick}");
            }
        }
    }
    let moved = h
        .canonical()
        .entities
        .iter()
        .find(|e| e.id == worker)
        .unwrap();
    assert_ne!(
        moved.order,
        bw_sim::Order::Hold,
        "it answers its player again"
    );
    assert!(h.net_view().away.is_none() && b.net_view().away.is_none());
    assert!(h.take_notices().iter().any(|n| n.ends_with("IS BACK")));
}

#[test]
fn a_seat_kept_open_is_dropped_when_its_time_is_up_and_cannot_come_back() {
    let options = LobbyOptions {
        fixed_delay: Some(3),
        auto: true,
        ..LobbyOptions::local()
    };
    let mut host_lobby = Lobby::host("127.0.0.1:0", Faction::Union, 13, options.clone()).unwrap();
    let code = JoinCode::parse(&host_lobby.local_addr().unwrap().to_string()).unwrap();
    let mut guest_lobby = Lobby::join(&code, None, options.clone()).unwrap();
    let (mut host, mut guest) = (None, None);
    let deadline = Instant::now() + Duration::from_secs(10);
    while (host.is_none() || guest.is_none()) && Instant::now() < deadline {
        host = host.or_else(|| host_lobby.poll());
        guest = guest.or_else(|| guest_lobby.poll());
        std::thread::sleep(Duration::from_millis(1));
    }
    let token = guest_lobby.token.expect("a rejoin token");
    let (mut host, mut guest) = (host.unwrap(), guest.unwrap());
    host.start_now();
    guest.start_now();
    run_to(&mut [&mut host, &mut guest], 60, Duration::from_secs(10));
    guest.crash();
    // A seat that is still heard cannot be kept open.
    assert!(!host.can_keep_open(1));
    step_until(
        &mut [&mut host],
        Duration::from_secs(10),
        "the host is offered to keep the seat",
        |s| s[0].net_view().can_keep_open,
    );
    host.set_keep_open_for(Duration::from_secs(2));
    host.keep_seat_open(1).unwrap();
    // The host plays on against a seat that holds still, which is not beaten
    // for being away...
    let kept = host.tick();
    run_to(&mut [&mut host], kept + 30, Duration::from_secs(10));
    assert!(host.canonical().outcome.is_none());
    assert!(all_hold(host.canonical(), 1));
    // ...until its time is up: then it is dropped and surrenders, as today.
    step_until(
        &mut [&mut host],
        Duration::from_secs(10),
        "the kept seat is dropped",
        |s| s[0].canonical().outcome.is_some(),
    );
    assert_eq!(host.canonical().outcome, Some(Outcome::Victory(0)));
    assert!(host.take_notices().iter().any(|n| n.ends_with("LEFT")));
    // Coming back now is refused, and says why.
    let mut late = Lobby::join(&code, Some(token), options).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let _ = host.try_step();
        assert!(late.poll().is_none(), "no seat to come back to");
        if let Phase::Failed(reason) = &late.phase {
            assert!(reason.contains("given up"), "{reason}");
            break;
        }
        assert!(Instant::now() < deadline, "no answer");
        std::thread::sleep(Duration::from_millis(2));
    }
}
