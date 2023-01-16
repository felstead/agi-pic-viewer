use std::{{fs::File}, io::{Read}, path::{Path}, time::{Instant}};
use eframe::egui;
use egui::*;
use egui::style::*;

use crate::agi_types::{common::*, resource::*, pic::*, game::*};
mod agi_types;

struct AgiViewerApp {
    fps : f32,
    _pointer_loc : Pos2,
    pic_shapes_list : Vec<Shape>,
    pic_rect : Rect,
    pri_shapes_list : Vec<Shape>,
    pri_rect : Rect,
    game : Game,
    selected_pic : usize,
    texture_handles : Vec<TextureHandle>,
    show_pri_buffer : bool,
    selected_instruction : usize
}

impl AgiViewerApp {
    fn new(game : Game) -> Self {
        let selected_instruction = if game.pic_resources.is_empty() { 0 } else { game.pic_resources[0].get_instructions().len() - 1 };
        AgiViewerApp {
            fps: 0.,
            _pointer_loc : Pos2::default(),
            pic_shapes_list : vec![],
            pic_rect : Rect::EVERYTHING,
            pri_shapes_list : vec![],
            pri_rect : Rect::EVERYTHING,
            game,
            selected_pic : 0,
            texture_handles : vec![],
            show_pri_buffer : false,
            selected_instruction
        }
    }

    fn get_selected_pic(&self) -> &PicResource{
        &self.game.pic_resources[self.selected_pic]
    }

    fn generate_view(&mut self, view : Rect, buffer_type : PictureBufferType, painter : &Painter) {

        let (buffer_view, shape_buffer, grid_stroke) = match buffer_type {
            PictureBufferType::Picture => (&mut self.pic_rect, &mut self.pic_shapes_list, Stroke { width: 1.0, color: Color32::from_rgb(0xdf,0xdf,0xdf) }),
            PictureBufferType::Priority => (&mut self.pri_rect, &mut self.pri_shapes_list, Stroke { width: 1.0, color: Color32::from_rgb(0xaf,0xaf,0xaf) })
        };

        let pic = &mut self.game.pic_resources[self.selected_pic].get_buffer(&buffer_type);

        // Redraw if the view rect has changed
        if *buffer_view != view {
            *buffer_view = view;

            let (x_step, y_step) = Self::get_xy_step(&view);

            shape_buffer.clear();

            // The actual image pixels
            Self::draw_pixels(buffer_view, shape_buffer, pic, painter);

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

    fn draw_pixels(view : &Rect, shape_buffer : &mut Vec<Shape>, pixels : &[FrameBufferPixel], painter : &Painter) {
        let (x_step, y_step) = Self::get_xy_step(view);

        for (i, px) in pixels.iter().enumerate() {
            let x = ((i % VIEWPORT_WIDTH) as f32 * x_step) + view.min.x;
            let y = ((i / VIEWPORT_WIDTH) as f32 * y_step) + view.min.y;

            let px_rect = Rect::from_min_size(painter.round_pos_to_pixels(pos2(x,y)), painter.round_vec_to_pixels(vec2(x_step, y_step)));

            shape_buffer.push(Shape::rect_filled(px_rect, Rounding::none(), PicResource::get_color(px.color)));
        }
    }

    fn get_xy_step(view : &Rect) -> (f32, f32) {
        (view.width() / VIEWPORT_WIDTH as f32, view.height() / VIEWPORT_HEIGHT as f32)
    }
}

impl eframe::App for AgiViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();

        if self.texture_handles.is_empty() {
            // Load the thumbnail textures
            self.game.pic_resources.iter().enumerate().for_each(|(i, r)| {
                let image_data = ColorImage {
                    size: [VIEWPORT_WIDTH, VIEWPORT_HEIGHT],
                    pixels: r.get_raster_data(&PictureBufferType::Picture).clone()
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
                                    self.pic_rect = Rect::NOTHING;
                                    self.pri_rect = Rect::NOTHING;
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
                        ui.set_max_width(200.);
                        ScrollArea::both().auto_shrink([false; 2]).show(ui, |ui| {
                            ui.label("Instruction list");
                            ui.separator();

                            for i in 0..(self.get_selected_pic().get_instructions().len()) {
                                let inst = self.get_selected_pic().get_instructions()[i].clone();
                                ui.style_mut().wrap = Some(false);
                                /*let label_response = ui.button(format!("{}. {:?}", i, inst));
                                if label_response.clicked() {
                                    self.game.pic_resources[self.selected_pic].render(i, false);

                                    // Hack to invalidate and redraw everything
                                    self.pic_rect = Rect::NOTHING;
                                    self.pri_rect = Rect::NOTHING;
                                }*/

                                let button = ui.selectable_value(&mut self.selected_instruction, i, format!("{}. {:?}", i, inst));
                                if button.clicked() {
                                    self.game.pic_resources[self.selected_pic].render(i, false);

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

                        let available_space = ui.available_size_before_wrap();

                        // Canvas selector tab
                        let button_container = ui.horizontal(|ui| {
                            ui.selectable_value(&mut self.show_pri_buffer, false, "Show Picture Buffer");
                            ui.selectable_value(&mut self.show_pri_buffer, true, "Show Priority Buffer");
                        });

                        let label_height = button_container.response.rect.height();

                        // Picture canvas
                        let canvas_size = vec2(available_space.x, available_space.y - label_height - 5.0);

                        Frame::canvas(ui.style()).rounding(Rounding::none()).inner_margin(Margin::default()).show(ui, |ui| {
                            let (response, painter) = ui.allocate_painter(canvas_size, Sense::click());
                            
                            let view = response.rect;

                            match self.show_pri_buffer {
                                false => {
                                    self.generate_view(view, PictureBufferType::Picture, &painter);
                                    painter.extend(self.pic_shapes_list.clone())
                                },
                                true => {
                                    self.generate_view(view, PictureBufferType::Priority, &painter);
                                    painter.extend(self.pri_shapes_list.clone());
                                }
                            }
                        });
                    });
                });
            });


        });

        let frame_time = now.elapsed().as_secs_f32();
        self.fps = 1. / frame_time;
    }
}


fn main() -> Result<(), AgiError> {

    
    //let resources = read_resources("PICDIR", &vol_file_data).unwrap();

    //let pic_resource = PicResource::new(resources[0].get_raw_data())?;

    //for resource in resources {}

    let game = Game::new_from_dir(Path::new("C:\\Program Files (x86)\\GOG Galaxy\\Games\\Kings Quest 2\\"))?;

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
