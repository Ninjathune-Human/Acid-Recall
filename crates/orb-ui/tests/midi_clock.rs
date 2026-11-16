use orb_ui::midi::{Config, preset::RoutingPreset, sync::Mode};

#[test]
fn old_preferences_disable_sync_and_routing_import_never_enables_a_master() {
    let mut config: Config = toml::from_str("enabled = true\npickup = true\n").unwrap();
    assert_eq!(config.clock.mode, Mode::Off);
    config.clock.mode = Mode::Slave;
    config.clock.input = Some("Drum machine IN".into());
    config.clock.output_delay_ms = 999;
    config.sanitize();
    assert_eq!(config.clock.output_delay_ms, 500);
    let preset = RoutingPreset::from_config(&config);
    let mut other = Config::default();
    preset.apply(&mut other).unwrap();
    assert_eq!(other.clock.mode, Mode::Off);
    assert!(other.clock.input.is_none());
    let encoded = toml::to_string(&config).unwrap();
    let restored: Config = toml::from_str(&encoded).unwrap();
    assert_eq!(restored.clock, config.clock);
}
