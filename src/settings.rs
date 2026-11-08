//! Settings, saved to `<game documents>/oxidizer/flight_panel.toml` (outside the mod folder, so updates keep them).

use oxidizer_api::{config_path, log};

pub struct Settings {
    pub visible: bool,
    pub show_orbit: bool,
    pub show_vehicle: bool,
    pub guard: bool,
    pub guard_max_speed: f32,
    pub guard_below_km: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { visible: true, show_orbit: true, show_vehicle: true, guard: true, guard_max_speed: 4.0, guard_below_km: 70.0 }
    }
}

impl Settings {
    pub fn load() -> Settings {
        let mut s = Settings::default();
        let Ok(text) = std::fs::read_to_string(config_path("flight_panel")) else { return s };
        let t: toml::Table = match text.parse() {
            Ok(t) => t,
            Err(e) => {
                log(&format!("flight_panel: settings file unreadable, using defaults: {e}"));
                return s;
            }
        };
        let flag = |k: &str, d: bool| t.get(k).and_then(|v| v.as_bool()).unwrap_or(d);
        let float = |k: &str, d: f64| t.get(k).and_then(|v| v.as_float().or(v.as_integer().map(|i| i as f64))).unwrap_or(d);
        s.visible = flag("visible", s.visible);
        s.show_orbit = flag("show_orbit", s.show_orbit);
        s.show_vehicle = flag("show_vehicle", s.show_vehicle);
        s.guard = flag("warp_guard", s.guard);
        s.guard_max_speed = float("warp_guard_max_speed", s.guard_max_speed as f64) as f32;
        s.guard_below_km = float("warp_guard_below_km", s.guard_below_km);
        s
    }

    pub fn save(&self) {
        let mut t = toml::Table::new();
        t.insert("visible".into(), self.visible.into());
        t.insert("show_orbit".into(), self.show_orbit.into());
        t.insert("show_vehicle".into(), self.show_vehicle.into());
        t.insert("warp_guard".into(), self.guard.into());
        t.insert("warp_guard_max_speed".into(), (self.guard_max_speed as f64).into());
        t.insert("warp_guard_below_km".into(), self.guard_below_km.into());
        if let Err(e) = std::fs::write(config_path("flight_panel"), t.to_string()) {
            log(&format!("flight_panel: could not save settings: {e}"));
        }
    }

    /// Hands the warp guard settings to the hooks.
    pub fn apply(&self) {
        crate::warp_guard::configure(self.guard, self.guard_max_speed as f64, self.guard_below_km * 1000.0);
    }
}
