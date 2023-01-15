use eframe::egui;
use egui::*;
use egui::style::*;
use std::time::{Instant};
use crate::agi_types::*;

pub fn render_window(pic : PicResource) {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let width = 1500.;
    let height = 900.;

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(width, height)),
        ..Default::default()
    };
    
    eframe::run_native(
        "AGI",
        options,
        Box::new(|_cc| Box::new(AgiViewerApp::new(pic))),
    )
}

struct AgiViewerApp {
    fps : f32,
    pointer_loc : Pos2,
    pic_grid_lines : Vec<Shape>,
    pic_rect : Rect,
    pri_grid_lines : Vec<Shape>,
    pri_rect : Rect,
    pic : PicResource
}

impl AgiViewerApp {
    fn new(pic : PicResource) -> Self {
        AgiViewerApp {
            fps: 0.,
            pointer_loc : Pos2::default(),
            pic_grid_lines : vec![],
            pic_rect : Rect::EVERYTHING,
            pri_grid_lines : vec![],
            pri_rect : Rect::EVERYTHING,
            pic
        }
    }

    fn generate_view(&mut self, view : Rect, buffer_type : PictureBufferType) {

        let (buffer_view, shape_buffer, grid_stroke) = match buffer_type {
            PictureBufferType::Picture => (&mut self.pic_rect, &mut self.pic_grid_lines, Stroke { width: 1.0, color: Color32::from_rgb(0xdf,0xdf,0xdf) }),
            PictureBufferType::Priority => (&mut self.pri_rect, &mut self.pri_grid_lines, Stroke { width: 1.0, color: Color32::from_rgb(0xaf,0xaf,0xaf) })
        };

        // Redraw if the view rect has changed
        if *buffer_view != view {
            *buffer_view = view;

            let (x_step, y_step) = Self::get_xy_step(&view);

            shape_buffer.clear();

            // The actual image pixels
            Self::draw_pixels(buffer_view, shape_buffer, self.pic.get_buffer(&buffer_type));

            // Grid lines
            let draw_grid = false;
            if draw_grid {
                for x in 0..VIEWPORT_WIDTH {
                    let l1 = pos2(x as f32 * x_step + view.min.x, view.min.y);
                    let l2 = pos2(x as f32 * x_step + view.min.x, view.max.y);
                    shape_buffer.push(Shape::line_segment([l1, l2], grid_stroke));
                }
    
                for y in 0..VIEWPORT_HEIGHT {
                    let l1 = pos2(view.min.x, y as f32 * y_step + view.min.y);
                    let l2 = pos2(view.max.x, y as f32 * y_step + view.min.y);
                    shape_buffer.push(Shape::line_segment([l1, l2], grid_stroke));
                }
            }
        }
    }

    fn draw_pixels(view : &Rect, shape_buffer : &mut Vec<Shape>, pixels : &Vec<FrameBufferPixel>) {
        let (x_step, y_step) = Self::get_xy_step(view);

        for (i, px) in pixels.iter().enumerate() {
            let x = ((i % VIEWPORT_WIDTH) as f32 * x_step) + view.min.x;
            let y = ((i / VIEWPORT_WIDTH) as f32 * y_step) + view.min.y;

            let px_rect = Rect::from_min_size(pos2(x,y), vec2(x_step, y_step));

            shape_buffer.push(Shape::rect_filled(px_rect, Rounding::none(), Self::get_color(px.color)));
        }
    }

    fn get_xy_step(view : &Rect) -> (f32, f32) {
        (view.width() / VIEWPORT_WIDTH as f32, view.height() / VIEWPORT_HEIGHT as f32)
    }

    fn get_color(agi_color : u8) -> Color32 {
        // From here: https://moddingwiki.shikadi.net/wiki/EGA_Palette
        match agi_color {
            0x00 => Color32::from_rgb(0x00,0x00,0x00), // black
            0x01 => Color32::from_rgb(0x00,0x00,0xAA), // blue
            0x02 => Color32::from_rgb(0x00,0xAA,0x00), // green
            0x03 => Color32::from_rgb(0x00,0xAA,0xAA), // cyan
            0x04 => Color32::from_rgb(0xAA,0x00,0x00), // red
            0x05 => Color32::from_rgb(0xAA,0x00,0xAA), // magenta
            0x06 => Color32::from_rgb(0xAA,0x55,0x00), // brown
            0x07 => Color32::from_rgb(0xAA,0xAA,0xAA), // light gray
            0x08 => Color32::from_rgb(0x55,0x55,0x55), // dark gray
            0x09 => Color32::from_rgb(0x55,0x55,0xFF), // light blue
            0x0A => Color32::from_rgb(0x55,0xFF,0x55), // light green
            0x0B => Color32::from_rgb(0x55,0xFF,0xFF), // light cyan
            0x0C => Color32::from_rgb(0xFF,0x55,0x55), // light red
            0x0D => Color32::from_rgb(0xFF,0x55,0xFF), // light magenta
            0x0E => Color32::from_rgb(0xFF,0xFF,0x55), // yellow
            0x0F => Color32::from_rgb(0xFF,0xFF,0xFF), // white
            _ => Color32::from_rgb(0xFF,0x00,0xFF),
        }
    }
}

impl eframe::App for AgiViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!("{:?}", self.pointer_loc));

            ui.horizontal_centered(|ui| {
                // Add the instruction list
                ui.vertical(|ui| {
                    ui.set_max_width(200.);
                    ScrollArea::both().auto_shrink([false; 2]).show(ui, |ui| {
                        for i in 0..(self.pic.get_instructions().len()) {
                            let inst = self.pic.get_instructions()[i].clone();
                            ui.style_mut().wrap = Some(false);
                            let label_response = ui.button(format!("{}. {:?}", i, inst));
                            if label_response.clicked() {
                                self.pic.render(i);

                                // Hack to invalidate and redraw everything
                                self.pic_rect = Rect::NOTHING;
                                self.pri_rect = Rect::NOTHING;
                            }
                        }
                    })
                });
                
                ui.vertical(|ui| {
                    /*if response.clicked() {
                        self.pointer_loc = response.interact_pointer_pos().unwrap();
                        self.pointer_loc.x = ((self.pointer_loc.x - response.rect.min.x) / x_step).floor();
                        self.pointer_loc.y = ((self.pointer_loc.y - response.rect.min.y) / y_step).floor();
                    }*/

                    // Picture buffer
                    let available_space = ui.available_size_before_wrap();
                    let label_response = ui.label("Picture buffer:");
                    let label_height = label_response.rect.height();

                    let canvas_size = vec2(available_space.x, (available_space.y - label_height - label_height) / 2.0 - 5.0);

                    Frame::canvas(ui.style()).rounding(Rounding::none()).inner_margin(Margin::default()).show(ui, |ui| {
                        let (response, painter) = ui.allocate_painter(canvas_size, Sense::click());
                        
                        let view = response.rect;

                        self.generate_view(view, PictureBufferType::Picture);
                        painter.extend(self.pic_grid_lines.clone());
                    });

                    // Priority buffer
                    ui.label("Priority buffer:");
                    Frame::canvas(ui.style()).rounding(Rounding::none()).inner_margin(Margin::default()).show(ui, |ui| {
                        let (response, painter) = ui.allocate_painter(canvas_size, Sense::click());
                        
                        let view = response.rect;

                        self.generate_view(view, PictureBufferType::Priority);
                        painter.extend(self.pri_grid_lines.clone());
                    });
                });
                // Add the drawing frames
            });
        });

        let frame_time = now.elapsed().as_secs_f32();
        self.fps = 1. / frame_time as f32;
    }
}