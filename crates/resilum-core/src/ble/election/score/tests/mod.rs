mod choosing;

use super::{Candidate, Facts, HostsWhileOnARouter, rank, worth_a_handover};

fn candidate(nth: u8, facts: Facts) -> Candidate {
    Candidate {
        who: [nth; 16],
        facts,
        can_host_at_all: true,
    }
}

fn plain() -> Facts {
    Facts {
        battery_percent: 50,
        ..Facts::default()
    }
}

#[test]
fn no_amount_of_a_lower_rank_ever_overturns_a_higher_one() {
    let everything_below_concurrency = Facts {
        p2p_and_sta_at_once: HostsWhileOnARouter::Refused,
        charging: true,
        battery_percent: 100,
        neighbours_heard: u8::MAX,
        ..Facts::default()
    };
    let one_step_of_concurrency = Facts {
        p2p_and_sta_at_once: HostsWhileOnARouter::NoOneCanSay,
        ..Facts::default()
    };
    let bare_uplink = Facts {
        has_an_uplink: true,
        p2p_and_sta_at_once: HostsWhileOnARouter::Refused,
        ..Facts::default()
    };
    let everything_below_an_uplink = Facts {
        p2p_and_sta_at_once: HostsWhileOnARouter::Confirmed,
        ..everything_below_concurrency
    };

    assert!(rank(&one_step_of_concurrency) > rank(&everything_below_concurrency));
    assert!(rank(&bare_uplink) > rank(&everything_below_an_uplink));
}

#[test]
fn a_phone_nobody_can_ask_outranks_one_that_said_no_but_not_one_that_said_yes() {
    let confirmed = Facts {
        p2p_and_sta_at_once: HostsWhileOnARouter::Confirmed,
        ..plain()
    };
    let unknown = Facts {
        p2p_and_sta_at_once: HostsWhileOnARouter::NoOneCanSay,
        ..plain()
    };
    let refused = Facts {
        p2p_and_sta_at_once: HostsWhileOnARouter::Refused,
        ..plain()
    };

    assert!(rank(&confirmed) > rank(&unknown));
    assert!(rank(&unknown) > rank(&refused));
}

#[test]
fn anything_at_a_wall_socket_outranks_anything_running_down_a_battery() {
    let at_a_socket = Facts {
        charging: true,
        battery_percent: 5,
        ..Facts::default()
    };
    let on_a_full_battery = Facts {
        charging: false,
        battery_percent: 100,
        neighbours_heard: u8::MAX,
        ..Facts::default()
    };

    assert!(rank(&at_a_socket) > rank(&on_a_full_battery));
}

#[test]
fn a_fuller_battery_wins_when_all_else_is_equal() {
    let fuller = Facts {
        battery_percent: 90,
        ..plain()
    };

    assert!(rank(&fuller) > rank(&plain()));
}

#[test]
fn a_battery_beyond_full_is_read_as_full_rather_than_beating_one() {
    let nonsense = Facts {
        battery_percent: 250,
        ..plain()
    };
    let full = Facts {
        battery_percent: 100,
        ..plain()
    };

    assert_eq!(rank(&nonsense), rank(&full));
}

#[test]
fn a_battery_or_a_neighbour_is_never_reason_enough_to_take_a_group_away() {
    let fuller = Facts {
        battery_percent: 100,
        neighbours_heard: 30,
        ..plain()
    };

    assert_eq!(worth_a_handover(&fuller), worth_a_handover(&plain()));
    assert!(rank(&fuller) > rank(&plain()));
}
