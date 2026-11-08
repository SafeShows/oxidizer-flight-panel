//! Warp guard: below a set altitude, time warp can't go above a set speed. Two Harmony patches on `KSA.Universe`:
//!
//! - a prefix on `IncreaseSimulationSpeed` that skips the game's method (returns false) when warping faster would
//!   pass the limit, and
//! - a postfix on `CanAutoWarp` that changes what the game gets back: false while the guard holds, so the game's
//!   auto-warp doesn't start.
//!
//! Hooks run inside the game's own calls, not in `frame`/`gui`, so they don't get `&mut self`. What they need lives
//! in atomics here; `frame` keeps the altitude current and the settings UI writes the limits.

use crate::game::universe;
use oxidizer_api::{Obj, hook, log, set_hook_result};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering::Relaxed};

static ENABLED: AtomicBool = AtomicBool::new(false);
static MAX_SPEED: AtomicU64 = AtomicU64::new(0); // f64 bits
static BELOW: AtomicU64 = AtomicU64::new(0); // meters, f64 bits
static ALTITUDE: AtomicU64 = AtomicU64::new(0); // NaN without a vehicle
static BLOCKED: AtomicU32 = AtomicU32::new(0);

fn load(a: &AtomicU64) -> f64 {
    f64::from_bits(a.load(Relaxed))
}

fn store(a: &AtomicU64, v: f64) {
    a.store(v.to_bits(), Relaxed)
}

/// Patches the game; call once from `init`.
pub fn install() {
    store(&ALTITUDE, f64::NAN);
    let ok = hook("KSA.Universe, KSA", "IncreaseSimulationSpeed", false, before_increase)
        & hook("KSA.Universe, KSA", "CanAutoWarp", true, after_can_auto_warp);
    if !ok {
        log("flight_panel: warp guard: could not patch the game (see above); the guard is off");
    }
}

pub fn configure(enabled: bool, max_speed: f64, below_m: f64) {
    ENABLED.store(enabled, Relaxed);
    store(&MAX_SPEED, max_speed);
    store(&BELOW, below_m);
}

/// The controlled vehicle's altitude, every frame (NaN without one).
pub fn set_altitude(m: f64) {
    store(&ALTITUDE, m);
}

/// True while the guard limits warp: on, and the vehicle is below the altitude.
pub fn holding() -> bool {
    ENABLED.load(Relaxed) && load(&ALTITUDE) < load(&BELOW)
}

/// How many warp increases the guard has refused.
pub fn blocked() -> u32 {
    BLOCKED.load(Relaxed)
}

fn before_increase(_universe: &Obj, _args: &Obj) -> bool {
    // The game steps through fixed speeds (1x, 2x, 5x, ...): stop at the step that reaches the limit.
    if holding() && universe::sim_speed() >= load(&MAX_SPEED) {
        BLOCKED.fetch_add(1, Relaxed);
        return false; // skip the game's IncreaseSimulationSpeed
    }
    true
}

fn after_can_auto_warp(_universe: &Obj, _args: &Obj) -> bool {
    if holding() {
        set_hook_result(&Obj::from(false));
    }
    true // ignored for postfixes
}
