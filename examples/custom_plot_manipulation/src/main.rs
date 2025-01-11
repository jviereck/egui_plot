//! This example shows how to implement custom gestures to pan and zoom in the plot
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use regex::Regex;

use std::{num::ParseIntError, ops::RangeBounds};

use eframe::egui::{self, DragValue, Event, Id, TextBuffer, Ui, Vec2, Vec2b};
use egui_plot::{Legend, Line, PlotPoints};

struct Data {
    name: String,
    path: String,
    index: i32,
    points: Vec<[f64; 2]>,
}

impl Data {
    fn create(name: &str) -> Data {
        let reg = Regex::new(r"^([^\[]+)\[(\d+)\]$").unwrap();
        if let Some(caps) = reg.captures(name) {
            let (_, [path_str, index_str]) = caps.extract();

            return Data {
                name: name.to_string(),
                path: path_str.to_string(),
                index: index_str.parse().unwrap(),
                points: Vec::new()
            }
        }
        Data {
            name: "FAILED TO PARSE".to_string(),
            path: "FAILED TO MATCH NAME".to_string(),
            index: -1,
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

                let from_str = caps.get(1).unwrap().as_str();
                from = from_str.parse().unwrap_or(from);
                to = match caps.get(2) {
                    Some(val) => &val.as_str()[1..],
                    None => from_str
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

    // Ignore whitespace and newlines. Each plot query is seperated by "|""
    let wo_newline = query.replace("\n", "");
    let wo_whitespace = wo_newline.replace(" ", "");
    let plot_queries = wo_whitespace.split("|");

    let mut plot_count = 0;
    let series: Vec<SeriesQuery> = plot_queries.enumerate().flat_map(|(i, layout)| {
        let plot_index = i as i32;
        plot_count += 1;

        println!("PlotIdx={} -> {}", plot_index, layout);

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
                let path_str = caps.get(1).unwrap().as_str().to_string();
                let range_str = match caps.get(2) {
                    Some(val) => &val.as_str()[1..val.len()-1],
                    None => "".as_str()
                };
                let ranges = parseSeriesProperty(range_str);
                plot_series.push(SeriesQuery {
                    plot_index,
                    path: path_str, // Ignore whitespace
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
            query: "trig[0] | trig[1:], pow[1] |pow[2:]".to_string(),
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
            let mut lin = Data::create("/data/pow[1]");
            let mut quat = Data::create("/data/pow[2]");
            let mut trip = Data::create("/data/pow[3]");

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
            self.data.push(cos);
            self.data.push(lin);
            self.data.push(quat);
            self.data.push(trip);

            self.plot_layout = parse(self.query.as_str());
        }

        egui::SidePanel::left("options").show(ctx, |ui| {
            let text: Vec<String> = self.data.iter().map(|d| {
                let mut res  = d.name.clone();
                res.insert_str(0, "* ");
                res
            }).collect();

            let mut content = "Available Data:\n".to_owned();
            content.push_str(text.join("\n").as_str());
            ui.label(content);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Plot layout:");

                    let response = ui.text_edit_multiline(&mut self.query);
                    response.ctx.input(|input| {
                        if input.key_pressed(egui::Key::Enter) {
                            self.plot_layout = parse(self.query.as_str());
                            println!("plot_count={}", self.plot_layout.plot_count);
                        }
                    });

                    // ui.text_edit_singleline(&mut self.timewindow);
                });

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let link_id = ui.id().with("linked_demo");

                    for plot_index in 0..self.plot_layout.plot_count {
                        // println!("plotting: {}", plot_index);
                        let plot = init_plot(ui.id().with(format!("plot{}", plot_index)), link_id);
                        plot.show(ui, |plot_ui| {
                            let last_bounds = plot_ui.plot_bounds();

                            // Get the data to display in this plot.
                            let plot_data = data_for_plot(
                                &self.data, &self.plot_layout, plot_index);

                            // Plot all the data that should go into this plot.
                            plot_data.iter().for_each(|data| {
                                // Filter data for the displayed range.
                                let mut fpoints = Vec::new();
                                for [x, y] in data.points.iter() {
                                    if *x >= last_bounds.min()[0] && *x <= last_bounds.max()[0] {
                                        fpoints.push([*x, *y]);
                                    }
                                }

                                plot_ui
                                    .line(Line::new(PlotPoints::new(fpoints))
                                    .name(data.name.as_str()));
                            });

                            plot_ui.set_auto_bounds([false, true].into());
                        });
                    }
                });
            });
        });
    }
}


fn data_in_range(data: &Data, series: &SeriesQuery) -> bool {
    series.ranges.iter().any(|r| {
        r.0 <= data.index && data.index <= r.1
    })
}

fn filter_data_for_series<'a>(data: &'a Vec<Data>, series: &SeriesQuery) -> Vec<&'a Data> {
    data.iter()
        .filter(|d| d.path.ends_with(series.path.as_str()))
        .filter(|d| data_in_range(d, series))
        .collect()
}

fn data_for_plot<'a>(data: &'a Vec<Data>, plot_layout: &PlotLayouts, plot_index: i32) -> Vec<&'a Data> {
    let plot_data: Vec<&Data> = plot_layout.series.iter()
        // Find the SeriesQueries that go into the current plot.
        .filter(|serie| serie.plot_index == plot_index)
        // Find the data for which the path matches the query path and index range.
        .flat_map(|serie| filter_data_for_series(data, serie))
        .collect();
    plot_data
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


    #[test]
    fn test_parse_a() {
        let res = parse("trig[0]");
        assert_eq!(res.plot_count, 1);
        assert_eq!(res.series.len(), 1);
        assert_eq!(res.series[0].ranges.len(), 1);
        let range = res.series[0].ranges[0];
        assert_eq!(range.0, 0);
        assert_eq!(range.1, 0);
    }

    #[test]
    fn test_data_for_plot() {
        let mut sin = Data::create("/data/trig[0]");
        let mut cos = Data::create("/data/trig[1]");

        let mut data = Vec::new();
        data.push(sin);
        data.push(cos);

        let plot_layout = parse("trig[0] | trig\n | trig[:]");

        {
            let plot_data = data_for_plot(
                &data, &plot_layout, 0);
            assert_eq!(plot_data.len(), 1);
        }

        {
            let plot_data = data_for_plot(
                &data, &plot_layout, 1);
            assert_eq!(plot_data.len(), 2);
        }

        {
            let plot_data = data_for_plot(
                &data, &plot_layout, 2);
            assert_eq!(plot_data.len(), 2);
        }

        // TODO: Add more tests.
    }
}
