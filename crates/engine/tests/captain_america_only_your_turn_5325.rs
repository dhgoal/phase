//! Regression for issue #5325 — Captain America, Living Legend.
//!
//! Oracle text (verbatim, verified from card-data.json):
//!   "Whenever a creature you control becomes tapped during your turn, if it's
//!    the first time that creature has become tapped this turn, untap it."
//!
//! The "during your turn" qualifier restricts the tap-watcher to the
//! controller's turn (CR 500.1 turn structure; CR 603.4 intervening-if). Before
//! the parser fix, that qualifier was dropped, so `TriggerConstraint` stayed
//! `None` and the untap fired on ANY player's turn — including opponents' turns
//! (the reported bug: Captain America untapped a creature that was tapped by an
//! opponent's effect on the opponent's turn).
//!
//! Both halves tap a creature YOU control via the SAME mechanism — an
//! opponent's "{T}: Tap target creature." activated ability. The only variable
//! is whose turn it is:
//!   * on YOUR turn  → the trigger fires and the creature is untapped
//!     (positive reach-guard: proves the tap path reaches the trigger and the
//!      first-tap intervening-if is satisfied).
//!   * on the OPPONENT's turn → the trigger is gated by `OnlyDuringYourTurn` and
//!     the creature STAYS tapped (the #5325 regression). Revert the parser fix
//!     and this half fails: the untap fires on the opponent's turn.

use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::types::game_state::WaitingFor;
use engine::types::phase::Phase;
use engine::types::player::PlayerId;

const CAPTAIN_AMERICA: &str = "Whenever a creature you control becomes tapped during your turn, \
     if it's the first time that creature has become tapped this turn, untap it.";

/// Build P0's Captain America + a P0 creature to be tapped (`victim`), and give
/// `actor` a "{T}: Tap target creature." source. `actor` is made the active
/// player with priority so its activation happens on its own turn. Returns the
/// runner plus the `victim` and `tapper` ids.
fn setup(
    actor: PlayerId,
) -> (
    GameRunner,
    engine::types::identifiers::ObjectId,
    engine::types::identifiers::ObjectId,
) {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);

    scenario
        .add_creature_from_oracle(P0, "Captain America, Living Legend", 4, 4, CAPTAIN_AMERICA)
        .id();
    let victim = scenario.add_creature(P0, "Grizzly Bear", 2, 2).id();
    let tapper = scenario
        .add_creature_from_oracle(actor, "Tapper", 1, 1, "{T}: Tap target creature.")
        .id();

    let mut runner = scenario.build();
    {
        let st = runner.state_mut();
        st.active_player = actor;
        st.priority_player = actor;
        st.waiting_for = WaitingFor::Priority { player: actor };
    }
    (runner, victim, tapper)
}

/// Positive reach-guard: on P0's own turn, the first tap of a creature P0
/// controls fires Captain America and untaps it. Proves the tap path used in
/// the negative half actually reaches the trigger.
#[test]
fn captain_america_untaps_on_your_turn() {
    let (mut runner, victim, tapper) = setup(P0);

    runner.activate(tapper, 0).target_object(victim).resolve();

    assert!(
        !runner.state().objects[&victim].tapped,
        "on your turn, Captain America untaps the creature on its first tap"
    );
}

/// The #5325 regression: a creature you control tapped by an opponent's effect
/// on the OPPONENT's turn must NOT be untapped — the "during your turn"
/// qualifier gates the trigger to your turn only.
#[test]
fn captain_america_does_not_untap_on_opponents_turn() {
    let (mut runner, victim, tapper) = setup(P1);

    runner.activate(tapper, 0).target_object(victim).resolve();

    assert!(
        runner.state().objects[&victim].tapped,
        "on the opponent's turn, Captain America must NOT untap the creature (#5325)"
    );
}
