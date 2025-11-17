use egui::{Rect, Pos2, Color32};

const BORDER_SIZE: f32 = 4.0;
const BORDER_COLOR: Color32 = Color32::from_rgb(96,0,0);
const PRESSED_KEY_COLOR: Color32 = Color32::from_rgb(64, 128, 255);

struct KeyCollision {
    key: usize,
    rect: Rect,
    black: bool,
}

pub struct KeyboardState {
    collision: Vec<KeyCollision>,
}

impl KeyboardState {
    pub fn new() -> Self {
        KeyboardState {
            collision: Vec::new(),
        }
    }
}

fn is_key_pressed(key: usize, keys: &[u8]) -> bool {
    keys.get(key).copied().unwrap_or(0) != 0
}

/*
        block0            block1
     _____/\_____   ________/\________
    /            \ /                  \
   |w0|w0|w0|w0|w0|w1|w1|w1|w1|w1|w1|w1|
    ____ _____ ___ ____ ____ _____ ____  _
   |  ||||  ||||  |  ||||  ||||  ||||  |  \
   |  ||||  ||||  |  ||||  ||||  ||||  |   \
   |  ||||  ||||  |  ||||  ||||  ||||  |    > row0
   |  ||||  ||||  |  ||||  ||||  ||||  |   /
   |  ||||  ||||  |  ||||  ||||  ||||  |  /
   |  +--+  +--+  |  +--+  +--+  +--+  | -
   |    |    |    |    |    |     |    |  \
   |    |    |    |    |    |     |    |   >  row1
   |    |    |    |    |    |     |    |  /
    ---- ---- ---- ---- ---- ----- ----  -
   | ww | ww | ww | ww | ww | ww  | ww |

w0 = block0_width / 5 = octave_width * 3/35
w1 = block1_width / 7 = octave_width * 4/49
ww = octave_width / 7

block0_width = octave_width * 3/7
block1_width = octave_width * 4/7
row0_height = octave_height * 5/8
row1_height = octave_height * 3/8

octave_width / octave_height = 13.6 / 8.2
*/

const OCTAVE_ASPECT_RATIO: f32 = 13.6 / 8.2;
const BLACK_KEY_HEIGHT: f32 = 5.0 / 8.0;

fn build_key_collision(keyboard_rect: Rect, state: &mut KeyboardState, first_key: usize) {
    let octave_height = keyboard_rect.height();
    let octave_width = octave_height * OCTAVE_ASPECT_RATIO;
    let ww = octave_width / 7.0;
    let w0 = octave_width * 3.0 / 35.0;
    let w1 = octave_width * 4.0 / 49.0;
    let row0_height = octave_height * BLACK_KEY_HEIGHT;
    state.collision.clear();
    for octave_n in 0..(keyboard_rect.width() / octave_width).ceil() as usize {
        let octave_x0 = keyboard_rect.min.x + octave_n as f32 * octave_width;

        // block 0 black keys
        for bk in (1..5).step_by(2) {
            let key_index = first_key + octave_n * 12 + bk;
            let ix = bk as f32;
            state.collision.push(KeyCollision {
                key: key_index,
                black: true,
                rect: Rect {
                    min: Pos2::new(octave_x0 + ix * w0, keyboard_rect.min.y),
                    max: Pos2::new(octave_x0 + (ix+1.0) * w0, keyboard_rect.min.y + row0_height),
                },
            });
        }

        // block 1 black keys
        for bk in (1..7).step_by(2) {
            let key_index = first_key + octave_n * 12 + 5 + bk;
            let ix = bk as f32;
            state.collision.push(KeyCollision {
                key: key_index,
                black: true,
                rect: Rect {
                    min: Pos2::new(octave_x0 + 3.0*ww + ix * w1, keyboard_rect.min.y),
                    max: Pos2::new(octave_x0 + 3.0*ww + (ix+1.0) * w1, keyboard_rect.min.y + row0_height),
                },
            });
        }

        // white keys
        for wk in 0..7 {
            let key_index = octave_n * 12 + wk * 2 - if wk > 2 { 1 } else { 0 };
            let x = octave_x0 + wk as f32 * octave_width / 7.0;
            state.collision.push(KeyCollision {
                key: first_key + key_index,
                black: false,
                rect: Rect {
                    min: Pos2::new(x, keyboard_rect.min.y),
                    max: Pos2::new(x + ww, keyboard_rect.max.y),
                },
            });
        }
    }
}

pub fn show_keyboard(ui: &mut egui::Ui, state: &mut KeyboardState, keys: &[u8]) {
    let size = ui.available_size();
    let (response, mut painter) = ui.allocate_painter(size, egui::Sense::drag());

    let keyboard_rect = Rect {
        min: Pos2 {
            x: response.rect.min.x,// + BORDER_SIZE,
            y: response.rect.min.y + BORDER_SIZE,
        },
        max: Pos2 {
            x: response.rect.max.x,// - BORDER_SIZE,
            y: response.rect.max.y,
        }
    };

    painter.rect_filled(response.rect, egui::CornerRadius::ZERO, BORDER_COLOR);
    painter.rect_filled(keyboard_rect, egui::CornerRadius::ZERO, Color32::WHITE);

    painter.shrink_clip_rect(keyboard_rect);
    build_key_collision(keyboard_rect, state, 36);

    let stroke = egui::Stroke::new(1.0, Color32::BLACK);

    // draw pressed white keys
    for col in &state.collision {
        if col.black { continue; }
        if col.rect.max.x > keyboard_rect.max.x { break; }
        if is_key_pressed(col.key, keys) {
            painter.rect_filled(col.rect, egui::CornerRadius::ZERO, PRESSED_KEY_COLOR);
        }
    }

    // draw white key divisions
    for col in &state.collision {
        if ! col.black {
            painter.vline(col.rect.min.x, col.rect.y_range(), stroke);
        }
    }

    // draw black keys
    for col in &state.collision {
        if col.rect.max.x > keyboard_rect.max.x {
            break;
        }
        if col.black {
            if is_key_pressed(col.key, keys) {
                painter.rect(col.rect, egui::CornerRadius::ZERO, PRESSED_KEY_COLOR, stroke, egui::StrokeKind::Inside);
            } else {
                painter.rect_filled(col.rect, egui::CornerRadius::ZERO, Color32::BLACK);
            }
        }
    }

    if let Some(pointer_pos) = response.interact_pointer_pos() {
        for col in &state.collision {
            if col.rect.contains(pointer_pos) {
                println!("on key: {}", col.key);
                break;
            }
        }
    }
}
