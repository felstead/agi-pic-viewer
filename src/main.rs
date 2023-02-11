use std::{{fs::File}, io::{Read}, path::{Path}, env};
use agi_types::pic_render::*;
use eframe::egui;
use egui::*;
use egui::style::*;

use crate::agi_types::{common::*, resource::*, pic::*, game::*};
mod agi_types;

#[derive(PartialEq)]
enum CanvasView {
    PicBufferPixels,
    PriBufferPixels,
    PicBufferVectors,
}

struct AgiViewerApp {
    _pointer_loc : Pos2,
    game : Game,
    canvas_view_rect : Rect,
    canvas_view_shapes : Vec<Shape>,
    selected_pic : usize,
    texture_handles : Vec<TextureHandle>,
    selected_canvas_view : CanvasView,
    selected_instruction : usize,
    line_width : f32,
    new_line_width : f32
}

impl AgiViewerApp {
    fn new(game : Game) -> Self {
        let selected_instruction = if game.pic_resources.is_empty() { 0 } else { game.pic_resources[0].get_instructions().len() - 1 };
        AgiViewerApp {
            _pointer_loc : Pos2::default(),
            game,
            canvas_view_rect : Rect::EVERYTHING,
            canvas_view_shapes : vec![],
            selected_pic : 0,
            texture_handles : vec![],
            selected_canvas_view : CanvasView::PicBufferPixels,
            selected_instruction,
            line_width : 2.0,
            new_line_width : 2.0
        }
    }

    fn get_selected_pic(&self) -> &PicResource{
        &self.game.pic_resources[self.selected_pic]
    }

    fn generate_view(pic : &PicResource, selected_instruction : usize, view : Rect, canvas_view_type : &CanvasView, line_width : f32, painter : &Painter) -> Vec<Shape> {

        let mut shape_buffer : Vec<Shape> = vec![];

        // The actual image pixels
        let (mut pic_buffer, mut pri_buffer, mut pic_vectors) = match canvas_view_type {
            CanvasView::PicBufferPixels => {
                (Some(PixelBuffer::new(get_color(PIC_BUFFER_BASE_COLOR))), None, None)
            },
            CanvasView::PriBufferPixels => {
                (None, Some(PixelBuffer::new(get_color(PIC_BUFFER_BASE_COLOR))), None)
            },
            CanvasView::PicBufferVectors => {
                (Some(PixelBuffer::new(get_color(PIC_BUFFER_BASE_COLOR))), None, Some(ShapeBuffer::new()))
            }
        };

        render_to_buffers(
            &pic.get_instructions()[0..=selected_instruction],
            &mut pic_buffer.as_mut(),
            &mut pri_buffer.as_mut(),
            &mut pic_vectors.as_mut()).unwrap();

        
        match canvas_view_type {
            CanvasView::PicBufferPixels => Self::draw_pixels(&view, &mut shape_buffer, pic_buffer.unwrap().get_pixels(), painter),
            CanvasView::PriBufferPixels => Self::draw_pixels(&view, &mut shape_buffer, pri_buffer.unwrap().get_pixels(), painter),
            CanvasView::PicBufferVectors => Self::draw_vectors(&view, line_width, &mut shape_buffer, &pic_vectors.unwrap(), painter)
        };

        shape_buffer
    }

    fn draw_pixels(view : &Rect, shape_buffer : &mut Vec<Shape>, pixels : &[Color32], painter : &Painter) {
        let (x_step, y_step) = Self::get_xy_step(view);

        for (i, px) in pixels.iter().enumerate() {
            let x = ((i % VIEWPORT_WIDTH) as f32 * x_step) + view.min.x;
            let y = ((i / VIEWPORT_WIDTH) as f32 * y_step) + view.min.y;

            let px_rect = Rect::from_min_max(painter.round_pos_to_pixels(pos2(x,y)), painter.round_pos_to_pixels(pos2(x+x_step, y+y_step)));

            shape_buffer.push(Shape::rect_filled(px_rect, Rounding::none(), *px));
        }
    }

    fn draw_vectors(view : &Rect, line_width : f32, shape_buffer : &mut Vec<Shape>, vectors : &ShapeBuffer, _painter : &Painter) {
        let (x_step, y_step) = Self::get_xy_step(view);

        // Background
        shape_buffer.push(Shape::rect_filled(*view, Rounding::none(), Color32::WHITE));

        // Add lines here
        for path in vectors.get_paths() {
            if path.points.len() == 1 {
                let p = path.points[0];
                shape_buffer.push(Shape::circle_filled(pos2((p.x * x_step) + view.min.x, (p.y * y_step) + view.min.y), line_width / 2.0, path.color));
            } else {
                let translated_lines = path.points.iter()
                .map(|p| {
                    pos2((p.x * x_step) + view.min.x, (p.y * y_step) + view.min.y)
                })
                .collect::<Vec<Pos2>>();

                let line = Shape::line(translated_lines, Stroke::new(line_width, path.color));

                shape_buffer.push(line);
            }
        }
    }

    fn get_xy_step(view : &Rect) -> (f32, f32) {
        (view.width() / VIEWPORT_WIDTH as f32, view.height() / VIEWPORT_HEIGHT as f32)
    }
}

impl eframe::App for AgiViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.texture_handles.is_empty() {
            // Load the thumbnail textures
            self.game.pic_resources.iter().enumerate().for_each(|(i, r)| {
                let mut pic_buffer = PixelBuffer::new(Color32::WHITE);
                render_to_buffers(r.get_instructions(), &mut Some(&mut pic_buffer), &mut None, &mut None).unwrap();

                let image_data = ColorImage {
                    size: [VIEWPORT_WIDTH, VIEWPORT_HEIGHT],
                    pixels: pic_buffer.get_pixels_vec()
                };
                self.texture_handles.push(ctx.load_texture(format!("PIC {}", i), image_data, Default::default()));
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered_justified(|ui| {
                // Thumbnails
                ScrollArea::horizontal().auto_shrink([true; 2]).show(ui, |ui| {
                    ui.set_max_height(150.);
                    ui.horizontal_centered(|ui| {
                        for (i, _resource) in self.game.pic_resources.iter().enumerate() {

                            ui.vertical(|ui| {
                                ui.style_mut().wrap = Some(false);
                                ui.label(format!("PIC {}", i));
                                let image_button = ImageButton::new(self.texture_handles[i].id(), vec2(VIEWPORT_WIDTH as f32, VIEWPORT_HEIGHT as f32 / 2.0))
                                    .selected(i == self.selected_pic);

                                if ui.add(image_button).clicked() {
                                    self.selected_pic = i;
                                    self.selected_instruction = self.game.pic_resources[i].get_instructions().len() - 1;
                                    
                                    // Hack to invalidate and redraw everything
                                    self.canvas_view_rect = Rect::NOTHING;
                                }
                                ui.set_max_width(VIEWPORT_WIDTH as f32 + 5.0);
                            });
                        }
                    });
                });

                // Main panel
                ui.horizontal_centered(|ui| {
                    // Add the instruction list
                    ui.vertical(|ui| {
                        ui.set_max_width(250.);
                        ScrollArea::both().auto_shrink([false; 2]).show(ui, |ui| {
                            ui.label("Instruction list");
                            ui.separator();

                            for i in 0..(self.get_selected_pic().get_instructions().len()) {
                                let inst_text = format!("{}. {}", i, self.get_selected_pic().get_instructions()[i]);
                                ui.style_mut().wrap = Some(false);
                                
                                let button = ui.selectable_value(&mut self.selected_instruction, i, inst_text);
                                if button.clicked() {
                                    // Invalidates the view for redraw
                                    self.canvas_view_rect = Rect::NOTHING;
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

                        let available_space = ui.available_size_before_wrap();

                        // Canvas selector tab
                        let button_container = ui.horizontal(|ui| {
                            let (a, b, c) = (
                                ui.selectable_value(&mut self.selected_canvas_view, CanvasView::PicBufferPixels, "Picture Buffer"),
                                ui.selectable_value(&mut self.selected_canvas_view, CanvasView::PicBufferVectors, "Upscaled Picture Buffer"),
                                ui.selectable_value(&mut self.selected_canvas_view, CanvasView::PriBufferPixels, "Priority Buffer"),
                            );

                            if a.clicked() || b.clicked() || c.clicked() {
                                // Invalidate
                                self.canvas_view_rect = Rect::NOTHING;
                            }

                            if self.selected_canvas_view == CanvasView::PicBufferVectors {
                                ui.add_space(10.0);
                                ui.add(Slider::new(&mut self.new_line_width, 0.0..=10.0).text("Line width"));

                                ui.add_space(10.0);
                                let (x, y) = Self::get_xy_step(&self.canvas_view_rect);
                                ui.label(format!("AGI pixel scale = ({:.1},{:.1})", x/2.0, y));
                            }
                            
                        });

                        let label_height = button_container.response.rect.height();

                        // Picture canvas
                        let canvas_size = vec2(available_space.x, available_space.y - label_height - 5.0);

                        Frame::canvas(ui.style()).rounding(Rounding::none()).inner_margin(Margin::default()).show(ui, |ui| {
                            let (response, painter) = ui.allocate_painter(canvas_size, Sense::click());
                            
                            let view = response.rect;

                            if response.rect != self.canvas_view_rect || self.new_line_width != self.line_width {
                                self.canvas_view_rect = response.rect;
                                self.line_width = self.new_line_width;
                                self.canvas_view_shapes = Self::generate_view(self.get_selected_pic(), self.selected_instruction, view, &self.selected_canvas_view, self.line_width, &painter);
                            }

                            painter.extend(self.canvas_view_shapes.clone());
                        });
                    });
                });
            });
        });
    }
}


fn main() -> Result<(), AgiError> {

    let args : Vec<_> = env::args().collect();

    if args.len() < 2 {
        println!("Please provide a path to an existing AGI game on your machine, e.g.:");
        println!("   agi-pic-viewer \"C:\\Program Files (x86)\\GOG Galaxy\\Games\\Kings Quest 2\\\"");
        return Ok(());
    }

    let game = Game::new_from_dir(Path::new(&args[1]))?;

    let width = 1500.;
    let height = 900.;

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(width, height)),
        ..Default::default()
    };
    
    eframe::run_native(
        format!("AGI Pic Viewer - {}", game.dir_name).as_str(),
        options,
        Box::new(|_cc| Box::new(AgiViewerApp::new(game))),
    );

    Ok(())
}
