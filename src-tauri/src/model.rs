use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeConfig {
    pub accent: String, pub background: String, pub foreground: String,
    pub field_background: String, pub muted_foreground: String,
    pub background_opacity: f64, pub content_opacity: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ThemeConfigFields {
    accent: String,
    background: String,
    foreground: String,
    #[serde(default)] field_background: OptionalColor,
    #[serde(default)] muted_foreground: OptionalColor,
    background_opacity: f64,
    content_opacity: f64,
}

#[derive(Debug)]
enum OptionalColor {
    Missing,
    Null,
    Value(String),
}

impl Default for OptionalColor {
    fn default() -> Self { Self::Missing }
}

impl<'de> Deserialize<'de> for OptionalColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match Option::<String>::deserialize(deserializer)? {
            Some(value) => Self::Value(value),
            None => Self::Null,
        })
    }
}

impl<'de> Deserialize<'de> for ThemeConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let fields = ThemeConfigFields::deserialize(deserializer)?;
        let field_background = match fields.field_background {
            OptionalColor::Value(value) => value,
            OptionalColor::Null => return Err(D::Error::custom("fieldBackground must be a string")),
            OptionalColor::Missing => derive_field_background(&fields.background).unwrap_or_default(),
        };
        let muted_foreground = match fields.muted_foreground {
            OptionalColor::Value(value) => value,
            OptionalColor::Null => return Err(D::Error::custom("mutedForeground must be a string")),
            OptionalColor::Missing => derive_muted_foreground(&fields.background, &fields.foreground).unwrap_or_default(),
        };
        Ok(Self {
            accent: fields.accent,
            background: fields.background,
            foreground: fields.foreground,
            field_background,
            muted_foreground,
            background_opacity: fields.background_opacity,
            content_opacity: fields.content_opacity,
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppSettings {
    pub layout_locked: bool, pub global_scale: f64, pub theme: ThemeConfig,
    pub show_desktop_mode: String, pub fade_enabled: bool,
    #[serde(default = "default_backup_interval_days")]
    pub backup_interval_days: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WidgetInstance {
    pub id: String,
    #[serde(rename = "type")] pub widget_type: String,
    pub enabled: bool, pub visible_wanted: bool,
    pub x: f64, pub y: f64, pub width: f64, pub height: f64,
    pub scale_mode: String, pub scale: f64,
    pub config: BTreeMap<String, serde_json::Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WidgetGeometry { pub x: f64, pub y: f64, pub width: f64, pub height: f64 }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayoutWorkArea { pub width: f64, pub height: f64 }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayoutProfile {
    pub work_area: LayoutWorkArea,
    pub widgets: BTreeMap<String, WidgetGeometry>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TodoItem { pub id: String, pub text: String, pub completed: bool }
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CountdownItem { pub id: String, pub name: String, pub target: f64 }
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WidgetContent { pub memo: String, pub todos: Vec<TodoItem>, pub countdowns: Vec<CountdownItem> }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSnapshot {
    pub state: String, pub last_success: Option<f64>, pub error: Option<String>,
    pub data: Option<serde_json::Value>,
}
impl Default for ProviderSnapshot {
    fn default() -> Self { Self { state: "idle".into(), last_success: None, error: None, data: None } }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Providers { pub codex: ProviderSnapshot, pub reset: ProviderSnapshot }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub revision: u64, pub settings: AppSettings, pub widgets: Vec<WidgetInstance>,
    pub content: BTreeMap<String, WidgetContent>,
    #[serde(default)] pub providers: Providers,
    #[serde(default)] pub active_layout_key: Option<String>,
    #[serde(default)] pub layout_profiles: BTreeMap<String, LayoutProfile>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum Action {
    Settings { settings: AppSettings },
    Widget { id: String, enabled: Option<bool>, scale_mode: Option<String>, scale: Option<f64> },
    Content { id: String, content: WidgetContent },
    Hide { id: String }, Show { id: String },
    ResetLayout { id: Option<String> }, Retry { provider: String },
}
pub const WIDGET_TYPES: [&str; 5] = ["memo", "todo", "countdown", "codex-reset", "codex-usage"];
fn default_backup_interval_days() -> u32 { 7 }
impl Default for AppSettings {
    fn default() -> Self {
        Self { layout_locked: false, global_scale: 1.0, show_desktop_mode: "auto".into(), fade_enabled: true,
            backup_interval_days: default_backup_interval_days(),
            theme: ThemeConfig {
                accent: "#82DDB8".into(), background: "#17212B".into(), foreground: "#EEF4F7".into(),
                field_background: derive_field_background("#17212B").expect("default background is valid"),
                muted_foreground: derive_muted_foreground("#17212B", "#EEF4F7").expect("default colors are valid"),
                background_opacity: 0.86, content_opacity: 1.0,
            } }
    }
}
impl Default for Snapshot {
    fn default() -> Self {
        let widgets = WIDGET_TYPES.iter().enumerate().map(|(i, kind)| WidgetInstance {
            id: (*kind).into(), widget_type: (*kind).into(), enabled: i < 2, visible_wanted: true,
            x: 64.0 + (i % 2) as f64 * 384.0, y: 72.0 + (i / 2) as f64 * 300.0,
            width: 352.0, height: 256.0, scale_mode: "inherit".into(), scale: 1.0, config: BTreeMap::new(),
        }).collect();
        Self { revision: 0, settings: AppSettings::default(), widgets,
            content: WIDGET_TYPES.iter().map(|k| ((*k).into(), WidgetContent::default())).collect(), providers: Providers::default(),
            active_layout_key: None, layout_profiles: BTreeMap::new() }
    }
}

fn in_range(value: f64, min: f64, max: f64) -> bool { value.is_finite() && value >= min && value <= max }
fn normalize_color(value: &str) -> Result<String, String> {
    let hex=value.trim().trim_start_matches('#');
    if hex.len()!=6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) || value.trim().starts_with("##") {
        return Err("invalid_color".into());
    }
    Ok(format!("#{}",hex.to_ascii_uppercase()))
}
fn color_channels(value: &str) -> Option<[u8; 3]> {
    let normalized = normalize_color(value).ok()?;
    Some([
        u8::from_str_radix(&normalized[1..3], 16).ok()?,
        u8::from_str_radix(&normalized[3..5], 16).ok()?,
        u8::from_str_radix(&normalized[5..7], 16).ok()?,
    ])
}
fn mix_colors(first: &str, first_weight: f64, second: &str, second_weight: f64) -> Option<String> {
    let first = color_channels(first)?;
    let second = color_channels(second)?;
    let channel = |index: usize| (first[index] as f64 * first_weight + second[index] as f64 * second_weight).round() as u8;
    Some(format!("#{:02X}{:02X}{:02X}", channel(0), channel(1), channel(2)))
}
fn derive_field_background(background: &str) -> Option<String> {
    mix_colors(background, 0.78, "#FFFFFF", 0.22)
}
fn derive_muted_foreground(background: &str, foreground: &str) -> Option<String> {
    mix_colors(background, 0.37, foreground, 0.63)
}
pub fn validate_settings(settings: &mut AppSettings) -> Result<(), String> {
    if !in_range(settings.global_scale,0.5,2.0)
        || !in_range(settings.theme.background_opacity,0.0,1.0)
        || !in_range(settings.theme.content_opacity,0.0,1.0)
        || !(1..=365).contains(&settings.backup_interval_days)
        || !["auto","follow-system"].contains(&settings.show_desktop_mode.as_str()) { return Err("invalid_settings".into()); }
    let accent = normalize_color(&settings.theme.accent)?;
    let background = normalize_color(&settings.theme.background)?;
    let foreground = normalize_color(&settings.theme.foreground)?;
    let field_background = normalize_color(&settings.theme.field_background)?;
    let muted_foreground = normalize_color(&settings.theme.muted_foreground)?;
    settings.theme.accent = accent;
    settings.theme.background = background;
    settings.theme.foreground = foreground;
    settings.theme.field_background = field_background;
    settings.theme.muted_foreground = muted_foreground;
    Ok(())
}
pub fn normalize_snapshot(state: &mut Snapshot) -> Result<(), String> {
    validate_settings(&mut state.settings)?;
    validate_snapshot(state)
}
fn valid_content(content: &WidgetContent) -> bool {
    if content.memo.len()>200_000 || content.todos.len()>1000 || content.countdowns.len()>1000 { return false; }
    let mut ids=std::collections::HashSet::new();
    for item in &content.todos { if item.id.is_empty() || item.id.len()>128 || item.text.len()>10_000 || !ids.insert(&item.id) { return false; } }
    ids.clear();
    content.countdowns.iter().all(|item| !item.id.is_empty() && item.id.len()<=128 && item.name.len()<=1000 && in_range(item.target,0.0,253402300799.0) && ids.insert(&item.id))
}
pub fn validate_snapshot(state: &Snapshot) -> Result<(), String> {
    validate_settings(&mut state.settings.clone())?;
    if state.widgets.len()!=WIDGET_TYPES.len() { return Err("invalid_widget_set".into()); }
    if state.content.len()!=state.widgets.len() { return Err("invalid_content".into()); }
    let mut ids=std::collections::HashSet::new();
    for widget in &state.widgets {
        if !WIDGET_TYPES.contains(&widget.id.as_str()) || widget.widget_type!=widget.id || !ids.insert(widget.id.clone())
            || !in_range(widget.scale,0.5,2.0) || !["inherit","custom"].contains(&widget.scale_mode.as_str())
            || !in_range(widget.width,120.0,10000.0) || !in_range(widget.height,100.0,10000.0)
            || !in_range(widget.x,-100000.0,100000.0) || !in_range(widget.y,-100000.0,100000.0) { return Err("invalid_widget".into()); }
        if !state.content.get(&widget.id).map(valid_content).unwrap_or(false) { return Err("invalid_content".into()); }
    }
    for provider in [&state.providers.codex, &state.providers.reset] {
        if !["idle", "loading", "ready", "offline", "error", "unavailable"].contains(&provider.state.as_str())
            || provider.last_success.is_some_and(|value| !value.is_finite() || value < 0.0) {
            return Err("invalid_provider".into());
        }
    }
    if state.layout_profiles.len() > 32 || state.active_layout_key.as_ref().is_some_and(|key| key.len() > 4096) { return Err("invalid_layout_profiles".into()); }
    for (key, profile) in &state.layout_profiles {
        if key.is_empty() || key.len() > 4096 || !in_range(profile.work_area.width, 1.0, 100_000.0) || !in_range(profile.work_area.height, 1.0, 100_000.0)
            || profile.widgets.len() != WIDGET_TYPES.len() { return Err("invalid_layout_profiles".into()); }
        for kind in WIDGET_TYPES {
            let Some(geometry) = profile.widgets.get(kind) else { return Err("invalid_layout_profiles".into()) };
            if !in_range(geometry.x,-100_000.0,100_000.0) || !in_range(geometry.y,-100_000.0,100_000.0)
                || !in_range(geometry.width,120.0,10_000.0) || !in_range(geometry.height,100.0,10_000.0) { return Err("invalid_layout_profiles".into()); }
        }
    }
    if state.active_layout_key.as_ref().is_some_and(|key| !state.layout_profiles.contains_key(key)) { return Err("invalid_layout_profiles".into()); }
    Ok(())
}
pub fn render_scale(widget: &WidgetInstance, settings: &AppSettings) -> f64 {
    settings.global_scale * if widget.scale_mode=="custom" { widget.scale } else { 1.0 }
}
pub fn apply_action(state: &mut Snapshot, caller: &str, action: Action) -> Result<(), String> {
    let owns=|id:&str| caller=="settings" || caller==format!("widget-{id}");
    match &action {
        Action::Content{id,..} | Action::Hide{id} => { if !owns(id) { return Err("permission_denied".into()); } }
        _ => { if caller!="settings" { return Err("permission_denied".into()); } }
    }
    let mut next=state.clone();
    match action {
        Action::Settings { mut settings } => { validate_settings(&mut settings)?; next.settings=settings; }
        Action::Content { id, content } => {
            if !next.widgets.iter().any(|w| w.id==id) || !valid_content(&content) { return Err("invalid_content".into()); }
            next.content.insert(id,content);
        }
        Action::Widget { id, enabled, scale_mode, scale } => {
            let widget=next.widgets.iter_mut().find(|w| w.id==id).ok_or("unknown_widget")?;
            if let Some(value)=enabled { widget.enabled=value; if value { widget.visible_wanted=true; } }
            if let Some(value)=scale_mode { widget.scale_mode=value; }
            if let Some(value)=scale { widget.scale=value; }
        }
        Action::Hide{id} => { next.widgets.iter_mut().find(|w|w.id==id).ok_or("unknown_widget")?.visible_wanted=false; }
        Action::Show{id} => {
            let widget=next.widgets.iter_mut().find(|w|w.id==id).ok_or("unknown_widget")?;
            if widget.enabled { widget.visible_wanted=true; }
        }
        Action::ResetLayout{id} => {
            if id.as_ref().is_some_and(|id| !next.widgets.iter().any(|w|&w.id==id)) { return Err("unknown_widget".into()); }
            for widget in &mut next.widgets {
                if id.as_ref().is_none_or(|id|id==&widget.id) {
                    let defaults=Snapshot::default();
                    let initial=defaults.widgets.iter().find(|w|w.id==widget.id).ok_or("unknown_widget")?;
                    widget.x=initial.x; widget.y=initial.y; widget.width=initial.width; widget.height=initial.height;
                }
            }
        }
        Action::Retry{provider} => {
            let item=match provider.as_str() { "codex"=>&mut next.providers.codex, "reset"=>&mut next.providers.reset, _=>return Err("unknown_provider".into()) };
            item.state="loading".into(); item.error=None;
        }
    }
    validate_snapshot(&next)?;
    crate::display_layout::capture_active(&mut next);
    next.revision=next.revision.checked_add(1).ok_or("revision_overflow")?;
    *state=next;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn own_content_edit_succeeds_but_cross_window_edit_fails() {
        let mut state = Snapshot::default();
        let mut content = WidgetContent::default(); content.memo = "记录中文".into();
        assert!(apply_action(&mut state, "widget-memo", Action::Content { id: "memo".into(), content: content.clone() }).is_ok());
        let saved = state.clone();
        assert!(apply_action(&mut state, "widget-todo", Action::Content { id: "memo".into(), content }).is_err());
        assert_eq!(saved, state);
    }
    #[test] fn settings_are_not_available_to_widget() {
        let mut state = Snapshot::default();
        assert!(apply_action(&mut state, "widget-memo", Action::Settings { settings: AppSettings::default() }).is_err());
    }
    #[test] fn hide_is_sticky_and_show_does_not_enable_disabled_widgets() {
        let mut s = Snapshot::default();
        apply_action(&mut s,"settings",Action::Hide{id:"memo".into()}).unwrap();
        assert!(!s.widgets[0].visible_wanted);
        apply_action(&mut s,"settings",Action::Show{id:"countdown".into()}).unwrap();
        assert!(!s.widgets[2].enabled);
    }
    #[test] fn reset_preserves_business_scale_and_enabled_state() {
        let mut s = Snapshot::default(); s.widgets[0].x=500.0; s.widgets[0].scale=1.5; s.content.get_mut("memo").unwrap().memo="Keep".into();
        apply_action(&mut s,"settings",Action::ResetLayout{id:Some("memo".into())}).unwrap();
        assert_eq!(s.widgets[0].x,64.0); assert_eq!(s.widgets[0].scale,1.5); assert_eq!(s.content["memo"].memo,"Keep");
    }
    #[test] fn valid_colors_normalize_and_invalid_settings_do_not_mutate() {
        let mut s=Snapshot::default(); let mut config=s.settings.clone(); config.theme.accent="aaffee".into();
        config.theme.background="abcdef".into(); config.theme.foreground="abc123".into();
        config.theme.field_background="deadbe".into(); config.theme.muted_foreground="fedcba".into();
        apply_action(&mut s,"settings",Action::Settings{settings:config}).unwrap();
        assert_eq!(s.settings.theme.accent,"#AAFFEE");
        assert_eq!(s.settings.theme.background,"#ABCDEF");
        assert_eq!(s.settings.theme.foreground,"#ABC123");
        assert_eq!(s.settings.theme.field_background,"#DEADBE");
        assert_eq!(s.settings.theme.muted_foreground,"#FEDCBA");
        let before=s.clone(); let mut bad=s.settings.clone(); bad.global_scale=f64::NAN;
        assert!(apply_action(&mut s,"settings",Action::Settings{settings:bad}).is_err()); assert_eq!(s,before);
    }
    #[test] fn legacy_theme_json_derives_missing_colors_and_preserves_legacy_fields() {
        let value = serde_json::json!({
            "accent": "#123abc",
            "background": "#17212B",
            "foreground": "#EEF4F7",
            "backgroundOpacity": 0.86,
            "contentOpacity": 1.0
        });
        let theme: ThemeConfig = serde_json::from_value(value).expect("legacy theme deserializes");
        assert_eq!(theme.accent, "#123abc");
        assert_eq!(theme.background, "#17212B");
        assert_eq!(theme.foreground, "#EEF4F7");
        assert_eq!(theme.field_background, "#4A525A");
        assert_eq!(theme.muted_foreground, "#9EA6AC");

        let explicit_field = serde_json::json!({
            "accent": "#123abc",
            "background": "#17212B",
            "foreground": "#EEF4F7",
            "backgroundOpacity": 0.86,
            "contentOpacity": 1.0,
            "fieldBackground": "#010203"
        });
        let theme: ThemeConfig = serde_json::from_value(explicit_field).expect("partial theme deserializes");
        assert_eq!(theme.field_background, "#010203");
        assert_eq!(theme.muted_foreground, "#9EA6AC");

        let explicit_muted = serde_json::json!({
            "accent": "#123abc",
            "background": "#17212B",
            "foreground": "#EEF4F7",
            "backgroundOpacity": 0.86,
            "contentOpacity": 1.0,
            "mutedForeground": "#030201"
        });
        let theme: ThemeConfig = serde_json::from_value(explicit_muted).expect("partial theme deserializes");
        assert_eq!(theme.field_background, "#4A525A");
        assert_eq!(theme.muted_foreground, "#030201");
    }
    #[test] fn explicit_null_theme_colors_are_rejected() {
        let value = serde_json::json!({
            "accent": "#123abc",
            "background": "#17212B",
            "foreground": "#EEF4F7",
            "backgroundOpacity": 0.86,
            "contentOpacity": 1.0,
            "fieldBackground": null
        });
        assert!(serde_json::from_value::<ThemeConfig>(value).is_err());
    }
    #[test] fn invalid_new_theme_colors_are_rejected_transactionally() {
        let mut state = Snapshot::default();
        let before = state.clone();
        let mut bad_field = state.settings.clone();
        bad_field.theme.field_background = "#12".into();
        assert!(apply_action(&mut state, "settings", Action::Settings { settings: bad_field }).is_err());
        assert_eq!(state, before);

        let mut bad_muted = state.settings.clone();
        bad_muted.theme.muted_foreground = "not-a-color".into();
        assert!(apply_action(&mut state, "settings", Action::Settings { settings: bad_muted }).is_err());
        assert_eq!(state, before);
    }
    #[test] fn serde_contract_uses_camel_case_and_action_tags() {
        let value=serde_json::to_value(Snapshot::default()).unwrap();
        assert_eq!(value["settings"]["showDesktopMode"],"auto");
        assert!(value["widgets"][0].get("visibleWanted").is_some());
        let action:Action=serde_json::from_str(r#"{"type":"widget","id":"memo","scaleMode":"custom","scale":1.5}"#).unwrap();
        assert!(matches!(action,Action::Widget { scale_mode:Some(_),.. }));
    }
    #[test] fn snapshot_rejects_content_for_unknown_widget() {
        let mut state = Snapshot::default();
        state.content.insert("unknown".into(), WidgetContent::default());
        assert_eq!(validate_snapshot(&state), Err("invalid_content".into()));
    }
    #[test] fn normalize_snapshot_updates_settings_in_place() {
        let mut state = Snapshot::default();
        state.settings.theme.accent = "aaffee".into();
        normalize_snapshot(&mut state).unwrap();
        assert_eq!(state.settings.theme.accent, "#AAFFEE");
    }
    #[test] fn snapshot_rejects_invalid_provider_state_and_timestamp() {
        let mut state = Snapshot::default();
        state.providers.codex.state = "bogus".into();
        assert_eq!(validate_snapshot(&state), Err("invalid_provider".into()));
        state.providers.codex.state = "ready".into();
        state.providers.codex.last_success = Some(-1.0);
        assert_eq!(validate_snapshot(&state), Err("invalid_provider".into()));
        state.providers.codex.last_success = Some(f64::NAN);
        assert_eq!(validate_snapshot(&state), Err("invalid_provider".into()));
    }
    #[test] fn backup_interval_defaults_for_legacy_settings_and_rejects_null_or_invalid_values() {
        let mut legacy = serde_json::to_value(AppSettings::default()).unwrap();
        legacy.as_object_mut().unwrap().remove("backupIntervalDays");
        let parsed: AppSettings = serde_json::from_value(legacy).unwrap();
        assert_eq!(parsed.backup_interval_days, 7);

        let mut null_value = serde_json::to_value(AppSettings::default()).unwrap();
        null_value["backupIntervalDays"] = serde_json::Value::Null;
        assert!(serde_json::from_value::<AppSettings>(null_value).is_err());

        let mut wrong_type = serde_json::to_value(AppSettings::default()).unwrap();
        wrong_type["backupIntervalDays"] = serde_json::json!("7");
        assert!(serde_json::from_value::<AppSettings>(wrong_type).is_err());
    }
    #[test] fn backup_interval_validation_accepts_only_one_to_365_days() {
        let mut settings = AppSettings::default();
        settings.backup_interval_days = 1;
        assert!(validate_settings(&mut settings).is_ok());
        settings.backup_interval_days = 365;
        assert!(validate_settings(&mut settings).is_ok());
        settings.backup_interval_days = 0;
        assert_eq!(validate_settings(&mut settings), Err("invalid_settings".into()));
        settings.backup_interval_days = 366;
        assert_eq!(validate_settings(&mut settings), Err("invalid_settings".into()));
    }
}
