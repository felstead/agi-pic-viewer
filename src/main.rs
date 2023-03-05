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
    thumbnail_texture_handles : Vec<TextureHandle>,
    main_viewport_texture : Option<TextureHandle>,
    selected_canvas_view : CanvasView,
    selected_instruction : usize,
    show_pixel_underlay : bool,
    line_width : f32,
    new_line_width : f32,
    render_options : RenderOptions
}

enum RenderData {
    Pixel(Vec<Color32>),
    Vector(Vec<Shape>),
    VectorAndPixel(Vec<Shape>, Vec<Color32>)

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
            thumbnail_texture_handles : vec![],
            main_viewport_texture : None,
            selected_canvas_view : CanvasView::PicBufferPixels,
            selected_instruction,
            show_pixel_underlay : false,
            line_width : 2.0,
            new_line_width : 2.0,
            render_options : RenderOptions { render_only_selected_instruction: false, show_fill_outlines: false }
        }
    }

    fn get_selected_pic(&self) -> &PicResource{
        &self.game.pic_resources[self.selected_pic]
    }

    fn generate_view(&self, pic : &PicResource, view : Rect, canvas_view_type : &CanvasView, line_width : f32, painter : &Painter) -> (Option<Vec<Color32>>, Option<Vec<Shape>>) {
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
            &pic.get_instructions()[0..=self.selected_instruction],
            &self.render_options,
            &mut pic_buffer.as_mut(),
            &mut pri_buffer.as_mut(),
            &mut pic_vectors.as_mut()).unwrap();
    
        
        let mut pixels : Option<Vec<Color32>> = None;
        let mut vectors : Option<Vec<Shape>> = None; 

        match canvas_view_type {
            CanvasView::PicBufferPixels => pixels = Some(pic_buffer.unwrap().get_pixels_vec()),
            CanvasView::PriBufferPixels => pixels = Some(pri_buffer.unwrap().get_pixels_vec()),
            CanvasView::PicBufferVectors => {
                if self.show_pixel_underlay {
                    pixels = Some(pic_buffer.unwrap().get_pixels_vec());
                }
                vectors = Some(Self::draw_vectors(&view, line_width, &pic_vectors.unwrap(), painter));
            }
        }
        (pixels, vectors)
    }

    fn draw_vectors(view : &Rect, line_width : f32, vectors : &ShapeBuffer, _painter : &Painter) -> Vec<Shape> {
        let (x_step, y_step) = Self::get_xy_step(view);

        let mut shape_buffer = vec![];

        let (px_offset_x, px_offset_y) = (x_step / 2f32, y_step / 2f32);

        // Add lines here
        for path in vectors.get_paths() {
            if path.points.len() == 1 {
                let p = path.points[0];
                shape_buffer.push(Shape::circle_filled(pos2((p.x * x_step) + view.min.x + px_offset_x, (p.y * y_step) + view.min.y + px_offset_y), line_width / 2.0, path.color));
            } else {
                let translated_lines = path.points.iter()
                .map(|p| {
                    pos2((p.x * x_step) + view.min.x + px_offset_x, (p.y * y_step) + view.min.y + px_offset_y)
                })
                .collect::<Vec<Pos2>>();

                let line = Shape::line(translated_lines, Stroke::new(line_width, path.color));

                shape_buffer.push(line);
            }
        }

        shape_buffer
    }

    fn get_xy_step(view : &Rect) -> (f32, f32) {
        (view.width() / VIEWPORT_WIDTH as f32, view.height() / VIEWPORT_HEIGHT as f32)
    }
}

impl eframe::App for AgiViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.thumbnail_texture_handles.is_empty() {
            // Load the thumbnail textures
            self.game.pic_resources.iter().enumerate().for_each(|(i, r)| {
                let mut pic_buffer = PixelBuffer::new(Color32::WHITE);
                render_to_buffers(r.get_instructions(), &RenderOptions::default(), &mut Some(&mut pic_buffer), &mut None, &mut None).unwrap();

                let image_data = ColorImage {
                    size: [VIEWPORT_WIDTH, VIEWPORT_HEIGHT],
                    pixels: pic_buffer.get_pixels_vec()
                };
                self.thumbnail_texture_handles.push(ctx.load_texture(format!("PIC {}", i), image_data, Default::default()));
            });
        }

        if self.main_viewport_texture.is_none() {
            // Blank image
            let pixels = Box::new([Color32::WHITE ; VIEWPORT_PIXELS]).to_vec();
            let blank = ColorImage {
                size: [VIEWPORT_WIDTH, VIEWPORT_HEIGHT],
                pixels
            };
            self.main_viewport_texture = Some(ctx.load_texture("MAIN_BUFFER", blank, Default::default()));
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
                                let image_button = ImageButton::new(self.thumbnail_texture_handles[i].id(), vec2(VIEWPORT_WIDTH as f32, VIEWPORT_HEIGHT as f32 / 2.0))
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

                        ui.label(format!("Instruction List ({}/{})", self.selected_instruction, self.get_selected_pic().get_instructions().len()));                        
                        ui.separator();
                        ScrollArea::both().auto_shrink([false; 2]).show(ui, |ui| {
                            for i in 0..(self.get_selected_pic().get_instructions().len()) {
                                let inst_text = format!("{}. {}", i, self.get_selected_pic().get_instructions()[i]);
                                ui.style_mut().wrap = Some(false);
                                
                                let button = ui.selectable_value(&mut self.selected_instruction, i, inst_text);
                                if button.clicked() {
                                    // Invalidates the view for redraw
                                    self.canvas_view_rect = Rect::NOTHING;
                                }
                            }
                        });
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

                            // Render options button
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.menu_button("Render Options ☰", |ui| {
                                    
                                    ui.vertical(|ui| {
                                        ui.set_width(200f32);
                                        ui.strong("Global Options");
                                        ui.separator();
                                        if ui.checkbox(&mut self.render_options.render_only_selected_instruction, "Render only selected instruction").clicked() {
                                            self.canvas_view_rect = Rect::NOTHING;
                                        };

                                        if ui.checkbox(&mut self.render_options.show_fill_outlines, "Show fill border lines").clicked() {
                                            self.canvas_view_rect = Rect::NOTHING;
                                        }

                                        ui.vertical(|ui| {
                                            ui.set_enabled(self.selected_canvas_view == CanvasView::PicBufferVectors);

                                            ui.separator();
                                            ui.strong("Vector Options");
                                            ui.separator();
                                            ui.label("Vector Line Width");
                                            ui.add(Slider::new(&mut self.new_line_width, 0.0..=10.0));
                                            let (x, y) = Self::get_xy_step(&self.canvas_view_rect);
                                            ui.label(format!("AGI pixel scale = ({:.1},{:.1})", x/2.0, y));
                                            ui.separator();
                                            if ui.checkbox(&mut self.show_pixel_underlay, "Show Pixel Underlay").clicked() {
                                                self.canvas_view_rect = Rect::NOTHING;
                                            };
                                        })
                                    });
                                });
                            });
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

                                let (pixels, vectors) = self.generate_view(self.get_selected_pic(), view, &self.selected_canvas_view, self.line_width, &painter);

                                self.canvas_view_shapes.clear();

                                if let Some(pixels) = pixels {
                                    let image_data = ColorImage {
                                        size: [VIEWPORT_WIDTH, VIEWPORT_HEIGHT],
                                        pixels
                                    };

                                    self.main_viewport_texture.as_mut().unwrap().set(image_data, TextureOptions::NEAREST);
                                    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));

                                    let tint = if self.show_pixel_underlay && self.selected_canvas_view == CanvasView::PicBufferVectors {
                                        Color32::from_white_alpha(128)
                                    } else {
                                        Color32::WHITE
                                    };

                                    self.canvas_view_shapes.push(Shape::image(self.main_viewport_texture.as_ref().unwrap().id(), view, uv, tint));
                                }

                                if let Some(vectors) = vectors {
                                    if !self.show_pixel_underlay {
                                        // If we're not showing the pixel underlay, draw the background
                                        self.canvas_view_shapes.push(Shape::rect_filled(view, Rounding::none(), Color32::WHITE));
                                    } else {
                                        // We are drawing the pixel underlay, so create an outline for our vectors
                                        let outlines : Vec<Shape> = vectors.iter().map(|v| {
                                            match v {
                                                Shape::Path(s) => Some(Shape::line(s.points.clone(), Stroke::new(self.line_width+1.5, Color32::WHITE))),
                                                _ => None
                                            }
                                        })
                                        .flatten()
                                        .collect();
                                        self.canvas_view_shapes.extend(outlines);
                                    }
                                    self.canvas_view_shapes.extend(vectors);
                                }
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

    let width = 1400.;
    let height = 800.;

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
