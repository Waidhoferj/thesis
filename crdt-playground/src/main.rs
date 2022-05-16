use std::{collections::HashMap, fmt::Display, ops::Deref};

use nannou::{
    prelude::*,
    rand::{thread_rng, Rng},
};
use shelf_crdt_macros::CRDT;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use shelf_crdt::adjacent_crdt::Doc;

fn main() {
    nannou::app(model).update(update).run();
}
#[derive(Clone, Copy)]
struct Effect {
    pos: Vec2,
    vel: Vec2,
    opacity: f32,
    rot: f32,
}

impl Effect {
    fn new(pos: Vec2) -> Self {
        let mut rng = thread_rng();
        let dir: Vec2 = rng.gen();
        let rot: f32 = rng.gen_range(-20.0..20.0);
        let vel = dir.normalize() * rng.gen_range(50.0..100.0);
        Effect {
            pos,
            vel,
            opacity: 1.0,
            rot,
        }
    }

    fn update(&mut self, delta: f32) {
        self.pos += self.vel * delta;
        self.rot += delta * 0.3;
        self.opacity -= delta * 0.4;
    }

    fn draw(&self, draw: &Draw) {
        draw.rect()
            .w_h(10.0, 10.0)
            .xy(self.pos)
            .color(hsla(
                self.pos.x * 0.01,
                self.pos.y * 0.01,
                0.5,
                self.opacity,
            ))
            .rotate(self.rot);
    }
}

#[derive(CRDT, Clone, Default, Serialize, Deserialize, Debug)]
struct MouseCursor {
    x: f32,
    y: f32,
}

impl From<Point2> for MouseCursor {
    fn from(point: Point2) -> Self {
        MouseCursor {
            x: point.x,
            y: point.y,
        }
    }
}

impl Display for MouseCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{x: {}, y: {}}}", self.x, self.y)
    }
}

impl From<MouseCursor> for Point2 {
    fn from(cursor: MouseCursor) -> Self {
        vec2(cursor.x, cursor.y)
    }
}

struct Model {
    _window: window::Id,
    id: String,
    shared_state: Doc<MouseCursorCRDT>, // TODO make this relative to the original
    effects: Vec<Effect>,
}

impl Model {
    fn set_mouse_pos(&mut self, point: Point2) {
        let cursor: MouseCursor = point.into();
        self.shared_state.update(&self.id, &cursor).unwrap();
    }

    fn get_mouse_pos(&self) -> Point2 {
        self.shared_state.get(&self.id).clone().into()
    }

    fn get_collaborator_mice(&self) -> Vec<Point2> {
        self.shared_state
            .elements
            .values()
            .map(|v| v.deref().clone().into())
            .collect()
    }

    // fn send_shared_state_update(&mut self) {
    //     let shelf = &self.shared_state;
    //     let sv: StateVector = shelf.into();
    //     let data = serde_json::to_vec(&sv).unwrap();
    //     self.communicator.send("update", &data);
    // }

    // fn receive_shared_state_update(&mut self) {
    //     while let Some(update) = self.communicator.try_recv() {
    //         // Use enum for the message type
    //         match update.topic.as_ref() {
    //             "update" => {
    //                 // state vector
    //                 let sv: StateVector = serde_json::from_slice(&update.data).unwrap();
    //                 if let Some(updates) = self.shared_state.get_state_delta(&sv) {
    //                     let data = serde_json::to_vec(&updates).unwrap();
    //                     self.communicator
    //                         .send(&format!("diff:{}", update.sender), &data);
    //                 }
    //             }
    //             "diff" => {
    //                 // delta / update
    //                 let update_shelf: Shelf<Value, Temporal> =
    //                     serde_json::from_slice(&update.data).unwrap();
    //                 // Change merge to just take self as mutable reference
    //                 let shared_state = std::mem::take(&mut self.shared_state);
    //                 self.shared_state = shared_state.merge(update_shelf);
    //             }
    //             _ => (),
    //         }
    //     }
    // }
}

fn model(app: &App) -> Model {
    let _window = app
        .new_window()
        .view(view)
        .mouse_moved(mouse_moved)
        .mouse_released(mouse_released)
        .key_released(on_key_release)
        .build()
        .unwrap();
    let id = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_micros()
        % 255;
    let id: String = id.to_string();
    let cursor_pos = MouseCursor { x: 0.0, y: 0.0 };
    let mut shared_state = Doc::default();
    shared_state.register(id.clone(), cursor_pos);

    Model {
        id,
        _window,
        effects: vec![],
        shared_state,
    }
}

fn update(_app: &App, model: &mut Model, update: Update) {
    let delta = update.since_last.as_secs_f32();
    for effect in model.effects.iter_mut() {
        effect.update(delta)
    }

    model.effects = model
        .effects
        .clone()
        .into_iter()
        .filter(|effect| effect.opacity > 0.0)
        .collect();

    model.shared_state.apply_updates().unwrap();
    model.shared_state.sync()
}

fn mouse_moved(_app: &App, model: &mut Model, pos: Point2) {
    model.set_mouse_pos(pos);
}

// fn mouse_pressed(_app: &App, _model: &mut Model, _button: MouseButton) {}

fn mouse_released(_app: &App, model: &mut Model, _button: MouseButton) {
    let mouse_position = model.get_mouse_pos();
    let new_effects = (0..10).map(|_| Effect::new(mouse_position));
    model.effects.extend(new_effects);
}

fn on_key_release(_app: &App, model: &mut Model, key: Key) {
    match key {
        Key::P => {
            let els: HashMap<String, MouseCursor> = model
                .shared_state
                .elements
                .iter()
                .map(|(k, v)| (k.clone(), v.deref().clone()))
                .collect();
            println!("{els:?}");
        }
        _ => (),
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    draw.background().color(PLUM);
    let x = model.get_collaborator_mice();
    x.into_iter().enumerate().for_each(|(i, pos)| {
        let hue = i as f32 / 20.0;
        let color = hsl(hue, 0.5, 0.5);
        draw.ellipse().color(color).radius(15.0).xy(pos);
    });

    model.effects.iter().for_each(|effect| effect.draw(&draw));

    draw.to_frame(app, &frame).unwrap();
}
