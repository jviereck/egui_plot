//! This example shows how to implement custom gestures to pan and zoom in the plot
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use regex::Regex;

use std::ops::RangeBounds;

use eframe::egui::{self, DragValue, Event, Id, Ui, Vec2, Vec2b};
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

static mut data: Vec<Data> = Vec::new();


struct PlotQuery {
    plot_index: i32,
    path: String,
    ranges: Vec<(i32, i32)>
}

struct PlotLayouts {
    plot_count: i32,
    queries: Vec<PlotQuery>
}

fn parse(query: String) -> PlotLayouts {
    let reg = Regex::new(r"^\s*([^\[,]+)(\[[^\]]*\])?,?\s*").unwrap();
    let plot_queries = query.split("|");
    let mut plot_count = 0;

    let all_queries: Vec<PlotQuery> = plot_queries.enumerate().flat_map(|(plot_index, entry)| {
        plot_count += 1;

        // The queries for this plot.
        let mut plot_queries: Vec<PlotQuery> = Vec::new();

        // While there is still more of the input to parse, keep going.
        let mut remain = String::from(entry);
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
                let all = caps.get(0).unwrap().as_str();
                // let path = caps.get(1).unwrap().as_str();
                // let range = caps.get(2).map_or("", |m| m.as_str());
                // println!("  {} -> {} @ {}", all, path, range);
                remain = remain.split_off(all.len());
            } else {
                // If no match and still bytes left in
                // `remain`, then got stuck with parsing.
                println!("Got stuck -> exit");
                break;
            }
        }
        return plot_queries;
    }).collect();

    return PlotLayouts {
        plot_count: plot_count,
        queries: all_queries
    }
}

fn main() /*-> eframe::Result*/ {
    // Create data.
    parse(String::from("test[1:2], foo | bar, baz[:3]"));

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

    // data.push(Data {
    //     entity_path: "/data/trig[0]".to_string(),
    //     points: Vec::new()
    // });



    // env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    // let options = eframe::NativeOptions::default();
    // eframe::run_native(
    //     "Plot",
    //     options,
    //     Box::new(|_cc| Ok(Box::<PlotExample>::default())),
    // )
}



struct PlotExample {
    query: String,
    lock_x: bool,
    lock_y: bool,
    ctrl_to_zoom: bool,
    shift_to_horizontal: bool,
    zoom_speed: f32,
    scroll_speed: f32,
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
        }
    }
}


impl eframe::App for PlotExample {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
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
            ui.horizontal(|ui| {
                let response = ui.text_edit_singleline(&mut self.query);
                response.ctx.input(|input| {
                    if input.key_pressed(egui::Key::Enter) {
                        parse(self.query.clone());
                    }
                });

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let link_id = ui.id().with("linked_demo");
                    plot(ui.id().with("plot0"), link_id);
                    plot(ui.id().with("plot1"), link_id);
                });
            });
        });
    }
}

fn plot<'a>(id_source: Id, link_id: Id) -> egui_plot::Plot<'a> {
    egui_plot::Plot::new(id_source)
        .set_margin_fraction([0., 0.05].into())
        .legend(Legend::default())
        .allow_drag([true, false])
        .allow_scroll([true, false])
        .allow_zoom([true, false])
        .height(400.0)
        .link_axis(link_id, [true, false])
        // .show(ui, |plot_ui| {
        //     let last_bounds = plot_ui.plot_bounds();

        //     let mut fpoints = Vec::new();
        //     for [x, y] in points.iter() {
        //         if *x >= last_bounds.min()[0] && *x <= last_bounds.max()[0] {
        //             fpoints.push([*x, *y]);
        //         }
        //     }

        //     plot_ui.line(Line::new(PlotPoints::new(fpoints)).name("Sine"));
        //     plot_ui.set_auto_bounds([false, true].into());
        // });
}
