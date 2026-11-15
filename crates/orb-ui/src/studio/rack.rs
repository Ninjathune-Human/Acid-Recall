//! Permanent sampler and effect column for the Graphite rack.

use eframe::egui::{self, Align2, Color32, FontId, Rect, Sense, Stroke, Ui, Vec2};
use orb_engine::studio::sampler::{Lane, Mode};
use orb_player::Command;

use super::editors::{trim_at, waveform};
use crate::app::App;

const ACID: Color32 = Color32::from_rgb(190, 215, 126);
const FACE: Color32 = Color32::from_rgb(29, 31, 32);

fn surface(ui: &mut Ui, rect: Rect, id: &str, draw: impl FnOnce(&mut Ui)) {
    ui.painter().rect_filled(rect, 5.0, FACE);
    ui.painter().rect_stroke(
        rect,
        5.0,
        Stroke::new(1.0, Color32::from_gray(55)),
        egui::StrokeKind::Inside,
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect.shrink(8.0))
            .id_salt(id),
        |ui| {
            ui.set_clip_rect(rect);
            let scale = (rect.width() / 300.0).clamp(0.8, 1.15);
            ui.style_mut().override_font_id = Some(FontId::proportional(12.0 * scale));
            ui.spacing_mut().item_spacing = Vec2::new(5.0 * scale, 3.0 * scale);
            ui.spacing_mut().interact_size.y = 18.0 * scale;
            ui.visuals_mut().selection.bg_fill = Color32::from_rgb(67, 78, 46);
            ui.visuals_mut().selection.stroke = Stroke::new(1.0, ACID);
            draw(ui);
        },
    );
}

fn float_knob(ui: &mut Ui, value: &mut f64, range: std::ops::RangeInclusive<f64>, label: &str) {
    let (lo, hi) = (*range.start(), *range.end());
    ui.vertical(|ui| {
        let width = (ui.available_width() / 3.0).clamp(32.0, 52.0);
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, width + 22.0), Sense::click_and_drag());
        if response.dragged() {
            let delta = ui.input(|i| -i.pointer.delta().y);
            let fine = ui.input(|i| if i.modifiers.shift { 0.1 } else { 1.0 });
            *value = (*value + f64::from(delta * fine) * (hi - lo) / 160.0).clamp(lo, hi);
        }
        let center = rect.center_top() + Vec2::new(0.0, width * 0.5);
        let radius = width * 0.34;
        ui.painter()
            .circle_filled(center, radius, Color32::from_gray(23));
        ui.painter()
            .circle_stroke(center, radius, Stroke::new(2.0, Color32::from_gray(83)));
        let angle = -std::f32::consts::PI * 1.25
            + ((*value - lo) / (hi - lo)) as f32 * std::f32::consts::PI * 1.5;
        let tip = center + Vec2::angled(angle) * radius * 0.82;
        ui.painter()
            .line_segment([center, tip], Stroke::new(2.0, ACID));
        ui.painter().text(
            rect.center_bottom() - Vec2::new(0.0, 13.0),
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(10.5),
            Color32::from_gray(200),
        );
        ui.painter().text(
            rect.center_bottom() - Vec2::new(0.0, 2.0),
            Align2::CENTER_BOTTOM,
            format!("{:.2}", *value),
            FontId::monospace(9.5),
            Color32::GRAY,
        );
        response.on_hover_text(format!(
            "{label}: drag vertically; Shift for fine adjustment"
        ));
    });
}

pub(super) fn gater_controls(ui: &mut Ui, lane: &mut Lane) {
    ui.horizontal_wrapped(|ui| {
        let mut on = lane.gater_source != 0;
        if ui.checkbox(&mut on, "GATER").changed() {
            lane.gater_source = if on { 4 } else { 0 };
        }
        egui::ComboBox::from_id_salt("gater-source")
            .width(66.0)
            .selected_text(match lane.gater_source {
                1 => "TB-A",
                2 => "TB-B",
                3 => "TR-808",
                _ => "TR-909",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut lane.gater_source, 3, "TR-808");
                ui.selectable_value(&mut lane.gater_source, 4, "TR-909");
            });
        let instruments = if lane.gater_source == 3 {
            [
                "BD", "SD", "LT", "MT", "HT", "RS", "CP", "CB", "CY", "OH", "CH",
            ]
        } else {
            [
                "BD", "SD", "LT", "MT", "HT", "RS", "CP", "CH", "OH", "CC", "RC",
            ]
        };
        let selected = usize::from(lane.gater_instrument.clamp(1, 11) - 1);
        egui::ComboBox::from_id_salt("gater-instrument")
            .width(42.0)
            .selected_text(instruments[selected])
            .show_ui(ui, |ui| {
                for (i, label) in instruments.iter().enumerate() {
                    ui.selectable_value(&mut lane.gater_instrument, i as u8 + 1, *label);
                }
            });
        let mut depth = lane.gater_depth * 100.0;
        if ui
            .add(
                egui::DragValue::new(&mut depth)
                    .range(0.0..=100.0)
                    .prefix("Depth ")
                    .suffix("%"),
            )
            .changed()
        {
            lane.gater_depth = depth / 100.0;
        }
        egui::ComboBox::from_id_salt("gater-hold")
            .width(73.0)
            .selected_text(format!("Hold {:.2}", lane.gater_hold))
            .show_ui(ui, |ui| {
                for (value, text) in [
                    (0.125, "1/128"),
                    (0.25, "1/64"),
                    (0.5, "1/32"),
                    (1.0, "1/16"),
                    (2.0, "1/8"),
                    (4.0, "1/4"),
                    (8.0, "1/2"),
                    (16.0, "1 bar"),
                ] {
                    ui.selectable_value(&mut lane.gater_hold, value, text);
                }
            });
        ui.add(
            egui::DragValue::new(&mut lane.gater_release_ms)
                .range(1.0..=500.0)
                .prefix("Smooth ")
                .suffix("ms"),
        );
        ui.checkbox(&mut lane.sidechain, "KICK SC").on_hover_text(
            "Sampler ducking follows the global kick sidechain enable, source and release",
        );
    });
}

impl App {
    pub(crate) fn graphite_sampler(&mut self, ui: &mut Ui, rect: Rect) {
        let before = self.studio.settings.clone();
        let mut load = None;
        let mut audition = false;
        let selected = self.studio.selected.min(7);
        let busy = self.studio.worker.is_some() || !self.studio.ready;
        let context = ui.ctx().clone();
        let tick = self.panel.transport.position.max(0) as usize;
        surface(ui, rect, "graphite-sampler", |ui| {
            egui::ScrollArea::vertical()
                .id_salt("sampler-rack-scroll")
                .max_height(ui.available_height())
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.colored_label(ACID, egui::RichText::new("SAMPLER").strong());
                        ui.small("8 LANES / 64 BARS");
                        if ui.add_enabled(!busy, egui::Button::new("LOAD")).clicked() {
                            load = Some(selected);
                        }
                        if ui
                            .add_enabled(
                                self.studio.waves[selected].is_some(),
                                egui::Button::new("AUDITION"),
                            )
                            .clicked()
                        {
                            audition = true;
                        }
                        if ui.button("EDIT / ALL LANES").clicked() {
                            self.studio.sampler_open = true;
                        }
                        ui.checkbox(&mut self.studio.follow, "Follow");
                        if busy {
                            ui.spinner();
                        }
                    });
                    ui.separator();
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(rect.width() * 0.14);
                            for i in 0..8 {
                                let name = format!("{}  {}", i + 1, self.studio.names[i]);
                                if ui
                                    .add_sized(
                                        [ui.available_width(), 16.0],
                                        egui::Button::new(name).selected(i == selected),
                                    )
                                    .clicked()
                                {
                                    self.studio.selected = i;
                                }
                            }
                        });
                        ui.vertical(|ui| {
                            ui.set_width((rect.width() * 0.82 - 16.0).max(100.0));
                            let lane = &mut self.studio.settings.lanes[selected];
                            if let Some(wave) = &self.studio.waves[selected] {
                                let response = waveform(
                                    ui,
                                    &wave.frames,
                                    (rect.height() * 0.25).max(32.0),
                                    lane.start,
                                    lane.end,
                                    self.studio.waveform_epoch,
                                    &mut self.studio.wave_views[selected],
                                );
                                let minimum = 2.0 / wave.frames.len().max(2) as f64;
                                if response.double_clicked() {
                                    lane.start = 0.0;
                                    lane.end = 1.0;
                                } else if let Some(p) = response.interact_pointer_pos() {
                                    let at = f64::from(
                                        (p.x - response.rect.left()) / response.rect.width(),
                                    )
                                    .clamp(0.0, 1.0);
                                    let id = response.id.with("handle");
                                    if response.drag_started() || response.clicked() {
                                        let entry =
                                            (at - lane.start).abs() <= (at - lane.end).abs();
                                        ui.data_mut(|d| d.insert_temp(id, entry));
                                    }
                                    if response.dragged() || response.clicked() {
                                        let entry =
                                            ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(true);
                                        trim_at(lane, entry, at, minimum);
                                    }
                                }
                                let duration = wave.frames.len() as f64 / 44100.0;
                                let mut entry = lane.start * duration;
                                let mut exit = lane.end * duration;
                                ui.horizontal(|ui| {
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut entry)
                                                .speed(0.001)
                                                .fixed_decimals(3)
                                                .prefix("IN ")
                                                .suffix("s"),
                                        )
                                        .changed()
                                    {
                                        trim_at(lane, true, entry / duration, minimum);
                                    }
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut exit)
                                                .speed(0.001)
                                                .fixed_decimals(3)
                                                .prefix("OUT ")
                                                .suffix("s"),
                                        )
                                        .changed()
                                    {
                                        trim_at(lane, false, exit / duration, minimum);
                                    }
                                });
                            } else {
                                ui.allocate_ui(
                                    Vec2::new(ui.available_width(), rect.height() * 0.28),
                                    |ui| {
                                        ui.centered_and_justified(|ui| {
                                            ui.label("LOAD WAV / AIFF / MP3");
                                        });
                                    },
                                );
                            }
                            ui.horizontal(|ui| {
                                for (mode, label) in [
                                    (Mode::OneShot, "ONE SHOT"),
                                    (Mode::AutoCut, "AUTO CUT"),
                                    (Mode::Gate, "GATE"),
                                    (Mode::Loop, "LOOP"),
                                ] {
                                    ui.selectable_value(&mut lane.mode, mode, label);
                                }
                                ui.add(
                                    egui::DragValue::new(&mut lane.pitch)
                                        .range(-24..=24)
                                        .prefix("Pitch "),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut lane.gain)
                                        .range(0.0..=2.0)
                                        .speed(0.005)
                                        .prefix("Level "),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut lane.send)
                                        .range(0.0..=1.0)
                                        .speed(0.005)
                                        .prefix("REV "),
                                );
                            });
                            gater_controls(ui, lane);
                            let bars = usize::from(lane.bars.clamp(1, 64));
                            if self.studio.follow && self.panel.transport.playing {
                                self.studio.bar = tick / 768 % bars;
                            }
                            self.studio.bar = self.studio.bar.min(bars - 1);
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(&mut lane.bars)
                                        .range(1..=64)
                                        .prefix("Length ")
                                        .suffix(" bars"),
                                );
                                let mut bar = self.studio.bar + 1;
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut bar)
                                            .range(1..=bars)
                                            .prefix("Bar "),
                                    )
                                    .changed()
                                {
                                    self.studio.bar = bar - 1;
                                    self.studio.follow = false;
                                }
                                ui.checkbox(&mut lane.muted, "M");
                                ui.checkbox(&mut lane.solo, "S");
                            });
                            ui.horizontal(|ui| {
                                let width = ((ui.available_width() - 15.0 * 3.0) / 16.0).max(8.0);
                                ui.spacing_mut().item_spacing.x = 3.0;
                                for step in 0..16 {
                                    let index = self.studio.bar * 16 + step;
                                    let on = lane.steps[index] != 0;
                                    let head = self.panel.transport.playing
                                        && tick / 48 % (usize::from(lane.bars) * 16) == index;
                                    let button = egui::Button::new(format!("{}", step + 1))
                                        .fill(if on {
                                            Color32::from_rgb(94, 112, 50)
                                        } else {
                                            FACE
                                        })
                                        .stroke(Stroke::new(
                                            if head { 2.0 } else { 1.0 },
                                            if head { ACID } else { Color32::from_gray(70) },
                                        ));
                                    let response = ui.add_sized([width, 19.0], button);
                                    if response.clicked() {
                                        lane.steps[index] = if on { 0 } else { 100 };
                                    }
                                    response.context_menu(|ui| {
                                        ui.add(
                                            egui::Slider::new(&mut lane.steps[index], 0..=127)
                                                .text("Velocity"),
                                        );
                                    });
                                }
                            });
                        });
                    });
                });
        });
        if self.studio.settings != before {
            self.studio.changed();
            self.doc.modified = true;
        }
        if let Some(lane) = load {
            self.studio_load_wave(&context, lane);
        }
        if audition {
            self.studio_command(Command::StudioAudition(selected));
        }
    }

    pub(crate) fn graphite_fx(&mut self, ui: &mut Ui, rect: Rect) {
        let before = self.studio.settings.clone();
        let width = rect.width();
        let gap = 7.0;
        let heights = [0.24, 0.14, 0.14, 0.14, 0.14, 0.20];
        let mut top = rect.top();
        for (i, fraction) in heights.into_iter().enumerate() {
            let card = Rect::from_min_size(
                egui::pos2(rect.left(), top),
                Vec2::new(width, rect.height() * fraction - gap),
            );
            top = card.bottom() + gap;
            surface(ui, card, &format!("graphite-fx-{i}"), |ui| match i {
                0 => {
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(self.studio.settings.enabled, "SPACE")
                            .clicked()
                        {
                            self.studio.settings.enabled = !self.studio.settings.enabled;
                        }
                        ui.small("CONVOLUTION");
                        if ui.small_button("...").clicked() {
                            self.studio.reverb_open = true;
                        }
                    });
                    ui.label(&self.studio.ir_name);
                    ui.horizontal(|ui| {
                        let s = &mut self.studio.settings;
                        float_knob(ui, &mut s.return_gain, 0.0..=1.0, "RETURN");
                        float_knob(ui, &mut s.predelay_ms, 0.0..=200.0, "PREDELAY");
                        float_knob(ui, &mut s.width, 0.0..=2.0, "WIDTH");
                    });
                    ui.horizontal(|ui| {
                        float_knob(
                            ui,
                            &mut self.studio.settings.low_cut,
                            20.0..=2000.0,
                            "LOW CUT",
                        );
                        float_knob(
                            ui,
                            &mut self.studio.settings.high_cut,
                            2000.0..=20000.0,
                            "HIGH CUT",
                        );
                        if ui.small_button("IR / SENDS").clicked() {
                            self.studio.reverb_open = true;
                        }
                    });
                }
                1..=4 => {
                    let (section, title, parameters): (&str, &str, &[(&str, &str)]) = match i {
                        1 => ("delay", "DELAY", &[("pan", "PAN"), ("feedback", "FDBK")]),
                        2 => ("dist", "DIST", &[("amount", "AMOUNT"), ("shape", "SHAPE")]),
                        3 => (
                            "pcf",
                            "PCF",
                            &[("freq", "FREQ"), ("q", "Q"), ("amt", "AMOUNT")],
                        ),
                        _ => (
                            "comp",
                            "COMP",
                            &[("threshold", "THRESH"), ("ratio", "RATIO")],
                        ),
                    };
                    ui.horizontal(|ui| {
                        let id = format!("{section}.on");
                        if ui
                            .selectable_label(self.panel.toggle_on(&id), title)
                            .clicked()
                        {
                            self.panel.press(&id, 0.0);
                        }
                        if ui.small_button("EDIT").clicked() {
                            self.studio.fx = Some(section.into());
                        }
                    });
                    ui.horizontal(|ui| {
                        for &(name, label) in parameters {
                            let id = format!("{section}.{name}");
                            if self.panel.value(&id).is_none() {
                                continue;
                            }
                            let old = self.panel.knob(&id);
                            let mut value = f64::from(old) / 1000.0;
                            ui.push_id(&id, |ui| {
                                float_knob(ui, &mut value, 0.0..=1.0, label);
                            });
                            let next = (value * 1000.0).round() as i32;
                            if next != old {
                                self.panel.set_knob(&id, next);
                            }
                        }
                    });
                }
                _ => {
                    ui.horizontal(|ui| {
                        ui.colored_label(ACID, "MASTER");
                        let on = self.panel.toggle_on("master.comp");
                        if ui.selectable_label(on, "COMP").clicked() {
                            self.panel.press("master.comp", 0.0);
                        }
                    });
                    let mut level = self.panel.fader("master.fader");
                    if ui
                        .add(egui::Slider::new(&mut level, 0..=999).text("LEVEL"))
                        .changed()
                    {
                        self.panel.set_fader("master.fader", level);
                    }
                    for key in ["master.meter_l", "master.meter_r"] {
                        let level = f32::from(*self.panel.meters.get(key).unwrap_or(&0)) / 31.0;
                        ui.add(egui::ProgressBar::new(level.clamp(0.0, 1.0)).desired_height(8.0));
                    }
                    ui.small("Stereo master / compressor before output");
                }
            });
        }
        if self.studio.settings != before {
            self.studio.changed();
            self.doc.modified = true;
        }
    }
}
