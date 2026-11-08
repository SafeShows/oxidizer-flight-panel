//! Flight Panel: an Oxidizer example mod in Rust. A window with live flight data, a warp guard that patches the
//! game, settings that are saved, hotkeys and Mods menu entries. API docs: https://docs.rs/oxidizer_api
//!
//! - game.rs: reading the game (typed wrappers over the by-name API)
//! - warp_guard.rs: Harmony patches that skip a game method and change a return value
//! - settings.rs: saving and loading settings
//! - this file: the window, hotkeys and menu

mod game;
mod settings;
mod warp_guard;

use game::{Vehicle, universe};
use oxidizer_api::{Mod, imgui, key_pressed, log};
use settings::Settings;

const AMBER: [f32; 4] = [1.0, 0.75, 0.2, 1.0];
const GREY: [f32; 4] = [0.6, 0.6, 0.6, 1.0];

struct FlightPanel {
    settings: Settings,
    /// Vertical speed comes from the altitude change over game time.
    last: Option<(f64, f64)>, // (game time, altitude)
    vertical_speed: f64,
    max_altitude: f64,
    max_speed: f64,
}

impl Mod for FlightPanel {
    fn init() -> Self {
        let settings = Settings::load();
        settings.apply();
        warp_guard::install();
        log("flight_panel: F7 shows the panel, F6 turns the warp guard on and off");
        FlightPanel { settings, last: None, vertical_speed: f64::NAN, max_altitude: 0.0, max_speed: 0.0 }
    }

    fn frame(&mut self, _player_time: f64, _dt: f64) {
        if key_pressed("F7") {
            self.toggle_panel();
        }
        if key_pressed("F6") {
            self.toggle_guard();
        }

        let Some(v) = Vehicle::controlled() else {
            warp_guard::set_altitude(f64::NAN);
            self.last = None;
            return;
        };
        let (t, alt) = (universe::time(), v.altitude());
        warp_guard::set_altitude(alt);
        if let Some((t0, alt0)) = self.last
            && t > t0
        {
            self.vertical_speed = (alt - alt0) / (t - t0);
        }
        if self.last.is_none_or(|(t0, _)| t != t0) {
            self.last = Some((t, alt)); // keep the old sample while the game is paused
        }
        self.max_altitude = self.max_altitude.max(alt);
        self.max_speed = self.max_speed.max(v.surface_speed());
    }

    fn menu(&mut self) {
        if imgui::menu_item("Flight panel  (F7)", self.settings.visible) {
            self.toggle_panel();
        }
        if imgui::menu_item("Warp guard  (F6)", self.settings.guard) {
            self.toggle_guard();
        }
    }

    fn gui(&mut self, _dt: f64) {
        if !self.settings.visible {
            return;
        }
        imgui::set_next_window_size(320.0, 520.0);
        let mut open = true;
        if imgui::begin_closable("Flight panel", &mut open) {
            match Vehicle::controlled() {
                Some(v) => self.draw_flight(&v),
                None => imgui::text_colored(GREY, "No vehicle"),
            }
            self.draw_guard();
            self.draw_settings();
        }
        imgui::end();
        if !open {
            self.toggle_panel(); // the window's close button
        }
    }
}

impl FlightPanel {
    fn toggle_panel(&mut self) {
        self.settings.visible = !self.settings.visible;
        self.settings.save();
    }

    fn toggle_guard(&mut self) {
        self.settings.guard = !self.settings.guard;
        self.settings.apply();
        self.settings.save();
        log(&format!("flight_panel: warp guard {}", if self.settings.guard { "on" } else { "off" }));
    }

    fn draw_flight(&self, v: &Vehicle) {
        imgui::text(&format!("{}  ({} at {})", v.name(), v.situation(), v.parent()));
        imgui::text(&format!("Warp {}x", num(universe::sim_speed(), 0)));
        imgui::separator();
        row("Altitude", &distance(v.altitude()));
        row("Above ground", &distance(v.radar_altitude()));
        row("Vertical speed", &speed(self.vertical_speed));
        row("Surface speed", &speed(v.surface_speed()));
        row("Orbital speed", &speed(v.orbital_speed()));

        if self.settings.show_orbit {
            imgui::separator();
            if v.has_orbit() {
                row("Apoapsis", &format!("{}  in {}", distance(v.apoapsis()), duration(v.time_to_apoapsis())));
                row("Periapsis", &format!("{}  in {}", distance(v.periapsis()), duration(v.time_to_periapsis())));
            } else {
                imgui::text_colored(GREY, "No orbit");
            }
        }
        if self.settings.show_vehicle {
            imgui::separator();
            let (mass, prop) = (v.total_mass(), v.propellant_mass());
            row("Mass", &format!("{} t", num(mass / 1000.0, 2)));
            row("Propellant", &format!("{} t  ({}%)", num(prop / 1000.0, 2), num(prop / mass * 100.0, 0)));
            row("Throttle", &format!("{}%", num(v.throttle() * 100.0, 0)));
        }
        imgui::separator();
        row("Highest", &distance(self.max_altitude));
        row("Fastest", &speed(self.max_speed));
    }

    fn draw_guard(&self) {
        imgui::separator();
        if !self.settings.guard {
            imgui::text_colored(GREY, "Warp guard off");
        } else if warp_guard::holding() {
            imgui::text_colored(AMBER, &format!("Warp guard: max {}x below {} km", self.settings.guard_max_speed, self.settings.guard_below_km));
        } else {
            imgui::text(&format!("Warp guard: on below {} km", self.settings.guard_below_km));
        }
        if warp_guard::blocked() > 0 {
            imgui::text_colored(GREY, &format!("Refused {} warp increases", warp_guard::blocked()));
        }
    }

    fn draw_settings(&mut self) {
        if !imgui::collapsing_header("Settings") {
            return;
        }
        let s = &mut self.settings;
        let mut changed = imgui::checkbox("Orbit", &mut s.show_orbit);
        changed |= imgui::checkbox("Vehicle", &mut s.show_vehicle);
        changed |= imgui::checkbox("Warp guard", &mut s.guard);
        changed |= imgui::slider_float("Max warp", &mut s.guard_max_speed, 1.0, 100.0);
        changed |= imgui::input_double("Below (km)", &mut s.guard_below_km);
        if changed {
            s.guard_max_speed = s.guard_max_speed.max(1.0);
            s.guard_below_km = s.guard_below_km.max(0.0);
            s.apply();
            s.save();
        }
        if imgui::button("Reset records") {
            self.max_altitude = 0.0;
            self.max_speed = 0.0;
        }
    }
}

fn row(label: &str, value: &str) {
    imgui::text(&format!("{label:<15}{value}"));
}

fn num(x: f64, decimals: usize) -> String {
    if x.is_finite() { format!("{x:.decimals$}") } else { "-".into() }
}

fn distance(m: f64) -> String {
    if m.abs() < 10_000.0 { format!("{} m", num(m, 0)) } else { format!("{} km", num(m / 1000.0, 1)) }
}

fn speed(ms: f64) -> String {
    format!("{} m/s", num(ms, 1))
}

fn duration(s: f64) -> String {
    if !s.is_finite() || s < 0.0 {
        return "-".into();
    }
    let s = s as u64;
    match (s / 3600, s / 60 % 60, s % 60) {
        (0, 0, sec) => format!("{sec}s"),
        (0, m, sec) => format!("{m}m {sec:02}s"),
        (h, m, sec) => format!("{h}h {m:02}m {sec:02}s"),
    }
}

oxidizer_api::oxidizer_mod!(FlightPanel);
