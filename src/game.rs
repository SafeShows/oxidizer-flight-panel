//! A small typed layer over the game, built on Oxidizer's by-name API (`Obj::get`, `Obj::call`).
//!
//! By-name calls find members when they run, so this keeps working across game updates as long as the names exist.
//! (Typed bindings from `oxidizer --bindgen` check members at compile time instead, but belong to one game build.)
//! Values that aren't available come back as NaN, and the panel shows them as "-".

use oxidizer_api::Obj;

fn num(o: Obj) -> f64 {
    o.f64().unwrap_or(f64::NAN)
}

/// `KSA.Vehicle`.
pub struct Vehicle(Obj);

impl Vehicle {
    /// The vehicle the player flies, if any.
    pub fn controlled() -> Option<Vehicle> {
        let v = Obj::static_get("KSA.Program, KSA", "ControlledVehicle");
        (!v.is_null()).then_some(Vehicle(v))
    }

    pub fn name(&self) -> String {
        self.0.get("Id").to_string()
    }

    /// Freefall, Landed, Floating, ... (the game's `Situation` enum).
    pub fn situation(&self) -> String {
        self.0.get("Situation").to_string()
    }

    /// The body it orbits, e.g. "Earth".
    pub fn parent(&self) -> String {
        self.0.get("Parent").get("Id").to_string()
    }

    /// Above sea level, in meters.
    pub fn altitude(&self) -> f64 {
        num(self.0.call("GetBarometricAltitude", &[]))
    }

    /// Above the ground below, in meters.
    pub fn radar_altitude(&self) -> f64 {
        num(self.0.call("GetRadarAltitude", &[]))
    }

    /// Relative to the ground, in m/s.
    pub fn surface_speed(&self) -> f64 {
        num(self.0.call("GetSurfaceSpeed", &[]))
    }

    /// Relative to the parent body's center, in m/s.
    pub fn orbital_speed(&self) -> f64 {
        num(self.0.get("OrbitalSpeed"))
    }

    pub fn has_orbit(&self) -> bool {
        self.0.get("HasMeaningfulOrbit").f64() == Some(1.0)
    }

    /// The parent body's mean radius, in meters.
    fn parent_radius(&self) -> f64 {
        num(self.0.get("Parent").get("MeanRadius"))
    }

    /// Altitude of the highest point of the orbit. (The game's `Apoapsis` is measured from the body's center.)
    pub fn apoapsis(&self) -> f64 {
        num(self.0.get("Apoapsis")) - self.parent_radius()
    }

    pub fn periapsis(&self) -> f64 {
        num(self.0.get("Periapsis")) - self.parent_radius()
    }

    /// Seconds of game time until the next apoapsis. (`NextApoapsisTime` is a game time: a `UniverseTime`.)
    pub fn time_to_apoapsis(&self) -> f64 {
        num(self.0.get("NextApoapsisTime").call("Seconds", &[])) - universe::time()
    }

    pub fn time_to_periapsis(&self) -> f64 {
        num(self.0.get("NextPeriapsisTime").call("Seconds", &[])) - universe::time()
    }

    /// In kg.
    pub fn total_mass(&self) -> f64 {
        num(self.0.get("TotalMass"))
    }

    pub fn propellant_mass(&self) -> f64 {
        num(self.0.get("PropellantMass"))
    }

    /// 0..1.
    pub fn throttle(&self) -> f64 {
        num(self.0.call("GetManualThrottle", &[]))
    }
}

/// `KSA.Universe`: game time and time warp.
pub mod universe {
    use oxidizer_api::Obj;

    /// Game time in seconds.
    pub fn time() -> f64 {
        super::num(Obj::static_call("KSA.Universe, KSA", "GetElapsedSeconds", &[]))
    }

    /// Time warp: 1 is real time.
    pub fn sim_speed() -> f64 {
        super::num(Obj::static_call("KSA.Universe, KSA", "GetSimulationSpeed", &[]))
    }
}
