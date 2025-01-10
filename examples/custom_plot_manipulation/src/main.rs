//! This example shows how to implement custom gestures to pan and zoom in the plot
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use regex::Regex;

use std::{num::ParseIntError, ops::RangeBounds};

use eframe::egui::{self, DragValue, Event, Id, TextBuffer, Ui, Vec2, Vec2b};
use egui_plot::{Legend, Line, PlotPoints};

struct Data {
    entity_path: String,
    points: Vec<[f64; 2]>,
}

impl Data {
    fn create(name: &str) -> Data {
        Data {
            entity_path: name.to_string(),
            points: Vec::new()
        }
    }

    fn add(&mut self, x: f64, y: f64) {
        self.points.push([x, y]);
    }
}


struct SeriesQuery {
    plot_index: i32,
    path: String,
    ranges: Vec<(i32, i32)>
}

struct PlotLayouts {
    plot_count: i32,
    series: Vec<SeriesQuery>
}

// TODO: Add feedback about failed parsing to returned values.
// At the moment this is interpreting the `[1:2]` part only as a slice operator -
// in the future this will become a general way to express properties of the
// plotted series.
fn parseSeriesProperty(def: &str) -> Vec<(i32, i32)> {
    // Ignore whitespaces.
    let mut res = Vec::new();

    // Ignore whitespaces (get removed).
    def.replace(" ", "").split(",")
        .for_each(|range_str| {
        // and color.
        let mut from: i32 = 0;
        let mut to: i32 = i32::MAX;

        // TODO: Add support for parsing series properties like line width

        // Empty string queries everything.
        if range_str.len() != 0 {
            let reg = Regex::new(r"^(\d*)(:\d*)?$").unwrap();
            if let Some(caps) = reg.captures(range_str) {
                println!("    {}", range_str);
                for (j, m) in caps.iter().enumerate() {
                    println!("      {}=>'{}'", j, match m {
                        Some(val) => val.as_str(),
                        None => "<no match>"
                    });
                }

                from = caps.get(1).unwrap().as_str().parse().unwrap_or(from);
                to = match caps.get(2) {
                    Some(val) => &val.as_str()[1..],
                    None => "".as_str()
                }.parse().unwrap_or(to);
            } else {
                println!("Got stuck -> exit");
                return;
            }
        }
        println!("    {}:{}", from, to);
        res.push((from, to));
    });
    res
}

fn parse(query: &str) -> PlotLayouts {
    let reg = Regex::new(r"^\s*([^\[,]+)(\[[^\]]*\])?,?\s*").unwrap(); // TODO: Make this a lazy-cell.
    let plot_queries = query.split("|");
    let mut plot_count = 0;

    let series: Vec<SeriesQuery> = plot_queries.enumerate().flat_map(|(i, layout)| {
        let plot_index = i as i32;
        plot_count += 1;

        // The queries for this plot.
        let mut plot_series: Vec<SeriesQuery> = Vec::new();

        // While there is still more of the input to parse, keep going.
        let mut remain = String::from(layout);
        while remain.len() > 0 {
            if let Some(caps) = reg.captures(&remain) {
                for (i, cap) in caps.iter().enumerate() {
                    if i == 0 {
                        println!("  {}", cap.unwrap().as_str());
                    } else {
                        println!("    {}", cap.map_or("<no-match>", |m| m.as_str()));
                    }
                }
                // // Dealing with variable change in matching groups.
                let path = caps.get(1).unwrap().as_str().to_string();
                let range_str = match caps.get(2) {
                    Some(val) => &val.as_str()[1..val.len()-1],
                    None => "".as_str()
                };
                let ranges = parseSeriesProperty(range_str);
                plot_series.push(SeriesQuery {
                    plot_index,
                    path,
                    ranges
                });

                // Cut off the consumed string.
                remain = remain.split_off(caps.get(0).unwrap().len());
            } else {
                // If no match and still bytes left in
                // `remain`, then got stuck with parsing.
                println!("Got stuck -> exit");
                break;
            }
        }
        return plot_series;
    }).collect();

    return PlotLayouts {
        plot_count,
        series
    }
}

mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_parse() {
        let res = parse("data/test[1:2], foo | taz[], all/that/is/there/toz[3],cat[4:]| bar, baz[:5,6:,7:8]");
        assert_eq!(res.plot_count, 3);
        assert_eq!(res.series.len(), 7);

        // TODO: Add more tests.
    }
}

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Plot",
        options,
        Box::new(|_cc| Ok(Box::<PlotExample>::default())),
    )
}



struct PlotExample {
    query: String,
    lock_x: bool,
    lock_y: bool,
    ctrl_to_zoom: bool,
    shift_to_horizontal: bool,
    zoom_speed: f32,
    scroll_speed: f32,
    plot_layout: PlotLayouts,
    data: Vec<Data>
}

impl Default for PlotExample {
    fn default() -> Self {
        Self {
            query: "".to_string(),
            lock_x: false,
            lock_y: false,
            ctrl_to_zoom: false,
            shift_to_horizontal: false,
            zoom_speed: 1.0,
            scroll_speed: 1.0,
            plot_layout: PlotLayouts {
                plot_count: 0,
                series: Vec::new()
            },
            data: Vec::new()
        }
    }
}

impl eframe::App for PlotExample {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if self.data.len() == 0 {
            // Create data.
            let mut sin = Data::create("/data/trig[0]");
            let mut cos = Data::create("/data/trig[1]");
            let mut lin = Data::create("/data/lin");
            let mut quat = Data::create("/data/pow[2]");
            let mut trip = Data::create("/data/pow[3]");;

            let mut x: f64 = 0.0;
            while x < 3.15 {
                sin.add(x, x.sin());
                cos.add(x, x.cos());
                lin.add(x, x);
                quat.add(x, x.powi(2));
                trip.add(x, x.powi(3));
                x += 0.01;
            }

            self.data.push(sin);
        }

        egui::SidePanel::left("options").show(ctx, |ui| {
            ui.checkbox(&mut self.lock_x, "Lock x axis").on_hover_text("Check to keep the X axis fixed, i.e., pan and zoom will only affect the Y axis");
            ui.checkbox(&mut self.lock_y, "Lock y axis").on_hover_text("Check to keep the Y axis fixed, i.e., pan and zoom will only affect the X axis");
            ui.checkbox(&mut self.ctrl_to_zoom, "Ctrl to zoom").on_hover_text("If unchecked, the behavior of the Ctrl key is inverted compared to the default controls\ni.e., scrolling the mouse without pressing any keys zooms the plot");
            ui.checkbox(&mut self.shift_to_horizontal, "Shift for horizontal scroll").on_hover_text("If unchecked, the behavior of the shift key is inverted compared to the default controls\ni.e., hold to scroll vertically, release to scroll horizontally");
            ui.horizontal(|ui| {
                ui.add(
                    DragValue::new(&mut self.zoom_speed)
                        .range(0.1..=2.0)
                        .speed(0.1),
                );
                ui.label("Zoom speed").on_hover_text("How fast to zoom in and out with the mouse wheel");
            });
            ui.horizontal(|ui| {
                ui.add(
                    DragValue::new(&mut self.scroll_speed)
                        .range(0.1..=100.0)
                        .speed(0.1),
                );
                ui.label("Scroll speed").on_hover_text("How fast to pan with the mouse wheel");
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                let response = ui.text_edit_singleline(&mut self.query);
                response.ctx.input(|input| {
                    if input.key_pressed(egui::Key::Enter) {
                        self.plot_layout = parse(self.query.as_str());
                        println!("plot_count={}", self.plot_layout.plot_count);
                    }
                });

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let link_id = ui.id().with("linked_demo");

                    for plot_index in 0..self.plot_layout.plot_count {
                        println!("plotting: {}", plot_index);
                        let plot = init_plot(ui.id().with(format!("plot{}", plot_index)), link_id);
                        plot.show(ui, |plot_ui| {
                            let last_bounds = plot_ui.plot_bounds();

                            let mut fpoints = Vec::new();
                            for [x, y] in self.data[0].points.iter() {
                                if *x >= last_bounds.min()[0] && *x <= last_bounds.max()[0] {
                                    fpoints.push([*x, *y]);
                                }
                            }

                            plot_ui.line(Line::new(PlotPoints::new(fpoints)).name("Sine"));
                            plot_ui.set_auto_bounds([false, true].into());
                        });
                    }
                });
            });
        });
    }
}

fn init_plot<'a>(id_source: Id, link_id: Id) -> egui_plot::Plot<'a> {
    egui_plot::Plot::new(id_source)
        .set_margin_fraction([0., 0.05].into())
        .legend(Legend::default())
        .allow_drag([true, false])
        .allow_scroll([true, false])
        .allow_zoom([true, false])
        .height(400.0)
        .link_axis(link_id, [true, false])
}
