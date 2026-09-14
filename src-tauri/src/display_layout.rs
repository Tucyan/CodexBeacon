use crate::model::{LayoutProfile, LayoutWorkArea, Snapshot, WidgetGeometry};
use std::{collections::BTreeMap, time::{Duration, Instant}};

pub type WorkArea = LayoutWorkArea;

#[derive(Debug, Clone, PartialEq)]
pub struct DisplayConfig { pub key: String, pub work_area: WorkArea }

#[derive(Debug, Clone, PartialEq)]
pub struct DisplayMonitor {
    name: String, x: i32, y: i32, width: u32, height: u32, scale_milli: u32, primary: bool,
}
impl DisplayMonitor {
    pub fn new(name: &str, x: i32, y: i32, width: u32, height: u32, scale: f64, primary: bool) -> Self {
        Self { name: name.into(), x, y, width, height, scale_milli: (scale * 1000.0).round().max(1.0) as u32, primary }
    }
}

pub fn topology_key(monitors: &[DisplayMonitor]) -> String {
    let mut monitors = monitors.to_vec();
    monitors.sort_by(|a,b| (a.x,a.y,&a.name,a.width,a.height,a.scale_milli,a.primary).cmp(&(b.x,b.y,&b.name,b.width,b.height,b.scale_milli,b.primary)));
    let mut key = String::from("display-v1");
    for monitor in monitors {
        key.push_str(&format!("|{}:{}:{},{},{}x{},{}:{}", monitor.name.len(), monitor.name, monitor.x, monitor.y, monitor.width, monitor.height, monitor.scale_milli, u8::from(monitor.primary)));
    }
    key
}

pub fn current_config(app: &tauri::AppHandle) -> Result<DisplayConfig, String> {
    let primary = app.primary_monitor().map_err(|_| "display_query_failed")?.ok_or("display_query_failed")?;
    let monitors = app.available_monitors().map_err(|_| "display_query_failed")?;
    let is_primary = |monitor: &tauri::Monitor| monitor.position() == primary.position() && monitor.size() == primary.size() && (monitor.scale_factor() - primary.scale_factor()).abs() < 0.001;
    let descriptors = monitors.iter().map(|monitor| DisplayMonitor::new(
        monitor.name().map(String::as_str).unwrap_or(""), monitor.position().x, monitor.position().y,
        monitor.size().width, monitor.size().height, monitor.scale_factor(), is_primary(monitor),
    )).collect::<Vec<_>>();
    let scale = primary.scale_factor().max(0.01);
    Ok(DisplayConfig {
        key: topology_key(&descriptors),
        work_area: WorkArea { width: primary.work_area().size.width as f64 / scale, height: primary.work_area().size.height as f64 / scale },
    })
}

pub fn fallback_config() -> DisplayConfig {
    let (left, top, right, bottom) = crate::window::native::work_area();
    DisplayConfig { key: format!("display-v1|fallback:{left},{top},{}x{}", right-left, bottom-top),
        work_area: WorkArea { width: (right-left).max(1) as f64, height: (bottom-top).max(1) as f64 } }
}

fn geometry(widget: &crate::model::WidgetInstance) -> WidgetGeometry {
    WidgetGeometry { x: widget.x, y: widget.y, width: widget.width, height: widget.height }
}

pub fn capture_active(snapshot: &mut Snapshot) {
    let Some(key) = snapshot.active_layout_key.clone() else { return };
    let Some(work_area) = snapshot.layout_profiles.get(&key).map(|profile| profile.work_area.clone()) else { return };
    let widgets = snapshot.widgets.iter().map(|widget| (widget.id.clone(), geometry(widget))).collect();
    snapshot.layout_profiles.insert(key, LayoutProfile { work_area, widgets });
}

fn current_widgets(snapshot: &Snapshot) -> BTreeMap<String, WidgetGeometry> {
    snapshot.widgets.iter().map(|widget| (widget.id.clone(), geometry(widget))).collect()
}

fn mapped_widgets(snapshot: &Snapshot, source: &BTreeMap<String, WidgetGeometry>, old: &WorkArea, new: &WorkArea) -> BTreeMap<String, WidgetGeometry> {
    snapshot.widgets.iter().map(|widget| {
        let source = source.get(&widget.id).cloned().unwrap_or_else(|| geometry(widget));
        let scale = crate::model::render_scale(widget, &snapshot.settings);
        let width = source.width.min((new.width / scale).max(120.0)).max(120.0);
        let height = source.height.min((new.height / scale).max(100.0)).max(100.0);
        let old_x_range = (old.width - source.width * scale).max(0.0);
        let old_y_range = (old.height - source.height * scale).max(0.0);
        let new_x_range = (new.width - width * scale).max(0.0);
        let new_y_range = (new.height - height * scale).max(0.0);
        let x = if old_x_range > 0.0 { (source.x / old_x_range).clamp(0.0, 1.0) * new_x_range } else { 0.0 };
        let y = if old_y_range > 0.0 { (source.y / old_y_range).clamp(0.0, 1.0) * new_y_range } else { 0.0 };
        (widget.id.clone(), WidgetGeometry { x, y, width, height })
    }).collect()
}

pub fn activate(snapshot: &mut Snapshot, config: &DisplayConfig) -> bool {
    if snapshot.active_layout_key.as_deref() == Some(config.key.as_str()) {
        capture_active(snapshot);
        let prior = snapshot.layout_profiles.get(&config.key).cloned().unwrap_or_else(|| LayoutProfile { work_area: config.work_area.clone(), widgets: current_widgets(snapshot) });
        let widgets = mapped_widgets(snapshot, &prior.widgets, &prior.work_area, &config.work_area);
        let updated = LayoutProfile { work_area: config.work_area.clone(), widgets };
        if prior == updated { return false; }
        for widget in &mut snapshot.widgets {
            if let Some(value) = updated.widgets.get(&widget.id) { widget.x=value.x;widget.y=value.y;widget.width=value.width;widget.height=value.height; }
        }
        snapshot.layout_profiles.insert(config.key.clone(),updated);return true;
    }
    capture_active(snapshot);
    let source_area = snapshot.active_layout_key.as_ref()
        .and_then(|key| snapshot.layout_profiles.get(key)).map(|profile| profile.work_area.clone())
        .unwrap_or_else(|| config.work_area.clone());
    let profile = if let Some(saved)=snapshot.layout_profiles.get(&config.key).cloned() {
        LayoutProfile { work_area: config.work_area.clone(), widgets: mapped_widgets(snapshot,&saved.widgets,&saved.work_area,&config.work_area) }
    } else {
        let source=current_widgets(snapshot);
        LayoutProfile { work_area: config.work_area.clone(), widgets: mapped_widgets(snapshot,&source,&source_area,&config.work_area) }
    };
    for widget in &mut snapshot.widgets {
        if let Some(value) = profile.widgets.get(&widget.id) {
            widget.x=value.x; widget.y=value.y; widget.width=value.width; widget.height=value.height;
        }
    }
    snapshot.layout_profiles.insert(config.key.clone(), profile);
    snapshot.active_layout_key = Some(config.key.clone());
    true
}

#[derive(Debug, Clone, PartialEq)]
pub enum DisplayDecision { Stable, Wait, Switch(DisplayConfig) }

pub struct DisplayTracker {
    active: DisplayConfig,
    candidate: Option<(DisplayConfig, Instant)>,
    settling_until: Option<Instant>,
}
impl DisplayTracker {
    pub fn new(active: DisplayConfig) -> Self { Self { active, candidate: None, settling_until: None } }
    #[cfg(test)] pub fn active_key(&self) -> &str { &self.active.key }
    pub fn is_active(&self, config:&DisplayConfig)->bool { &self.active==config }
    pub fn observe(&mut self, config: DisplayConfig, now: Instant) -> DisplayDecision {
        if config == self.active {
            self.candidate = None;
            if self.settling_until.is_some_and(|until| now < until) { return DisplayDecision::Wait; }
            self.settling_until = None;
            return DisplayDecision::Stable;
        }
        match &self.candidate {
            Some((candidate, since)) if candidate == &config && now.duration_since(*since) >= Duration::from_millis(750) => {
                DisplayDecision::Switch(config)
            }
            Some((candidate, _)) if candidate == &config => DisplayDecision::Wait,
            _ => { self.candidate = Some((config, now)); DisplayDecision::Wait }
        }
    }
    pub fn commit(&mut self, config: DisplayConfig, now: Instant) {
        self.active=config;self.candidate=None;self.settling_until=Some(now+Duration::from_millis(750));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Snapshot;
    use std::time::{Duration, Instant};

    fn config(key: &str, width: f64, height: f64) -> DisplayConfig {
        DisplayConfig { key: key.into(), work_area: WorkArea { width, height } }
    }

    #[test]
    fn a_new_display_inherits_current_layout_and_switching_back_restores_each_profile() {
        let mut snapshot = Snapshot::default();
        snapshot.widgets[0].x = 900.0;
        snapshot.widgets[0].y = 400.0;
        assert!(activate(&mut snapshot, &config("screen-a", 1920.0, 1040.0)));

        snapshot.widgets[0].x = 1000.0;
        capture_active(&mut snapshot);
        assert!(activate(&mut snapshot, &config("screen-b", 1280.0, 680.0)));
        assert!(snapshot.widgets[0].x < 1000.0, "new screen inherits a mapped visible position");

        snapshot.widgets[0].x = 120.0;
        capture_active(&mut snapshot);
        assert!(activate(&mut snapshot, &config("screen-a", 1920.0, 1040.0)));
        assert_eq!(snapshot.widgets[0].x, 1000.0);
        assert_eq!(snapshot.layout_profiles.len(), 2);
    }

    #[test]
    fn topology_key_distinguishes_dpi_and_monitor_arrangement() {
        let base = vec![DisplayMonitor::new("A", 0, 0, 1920, 1080, 1.0, true)];
        let scaled = vec![DisplayMonitor::new("A", 0, 0, 1920, 1080, 1.25, true)];
        let extended = vec![
            DisplayMonitor::new("A", 0, 0, 1920, 1080, 1.0, true),
            DisplayMonitor::new("B", 1920, 0, 2560, 1440, 1.0, false),
        ];
        assert_ne!(topology_key(&base), topology_key(&scaled));
        assert_ne!(topology_key(&base), topology_key(&extended));
    }

    #[test]
    fn tracker_suppresses_geometry_until_a_changed_display_is_stable() {
        let start = Instant::now();
        let mut tracker = DisplayTracker::new(config("screen-a", 1920.0, 1040.0));
        assert_eq!(tracker.observe(config("screen-b", 1280.0, 680.0), start), DisplayDecision::Wait);
        assert_eq!(tracker.observe(config("screen-b", 1280.0, 680.0), start + Duration::from_millis(500)), DisplayDecision::Wait);
        let candidate = match tracker.observe(config("screen-b", 1280.0, 680.0), start + Duration::from_millis(900)) {
            DisplayDecision::Switch(candidate) => candidate,
            other => panic!("expected switch, got {other:?}"),
        };
        assert_eq!(tracker.active_key(), "screen-a", "a failed layout/window update must leave the old topology active");
        tracker.commit(candidate, start + Duration::from_millis(900));
        assert_eq!(tracker.active_key(), "screen-b");
    }

    #[test]
    fn restored_and_resized_work_areas_keep_the_whole_widget_visible() {
        let mut snapshot = Snapshot::default();
        activate(&mut snapshot, &config("screen-a", 1920.0, 1040.0));
        snapshot.widgets[0].x = 1500.0;
        snapshot.widgets[0].y = 800.0;
        snapshot.widgets[0].width = 900.0;
        snapshot.widgets[0].height = 700.0;
        capture_active(&mut snapshot);
        activate(&mut snapshot, &config("screen-b", 2560.0, 1400.0));
        assert!(activate(&mut snapshot, &config("screen-a", 1280.0, 680.0)));
        let widget = &snapshot.widgets[0];
        let scale = crate::model::render_scale(widget, &snapshot.settings);
        assert!(widget.x >= 0.0 && widget.x + widget.width * scale <= 1280.001);
        assert!(widget.y >= 0.0 && widget.y + widget.height * scale <= 680.001);
    }
}
