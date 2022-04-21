use std::{mem::replace, ops::Deref};

use nannou::{
    prelude::*,
    rand::{thread_rng, Rng},
};
use std::time::SystemTime;

use networking::Multicast;
use serde_json::{json, Value as JSON};
use shelf_crdt::temporal::Temporal;
use shelf_crdt::wrap_crdt::{Atomic, Shelf, StateVector, Value};
use shelf_crdt::{DeltaCRDT, Mergeable};

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

struct Model {
    _window: window::Id,
    id: u8,
    shared_state: Shelf<Value, Temporal>,
    effects: Vec<Effect>,
    communicator: Multicast,
    last_update_sent: u128,
    update_frequency: usize,
}

impl Model {
    fn set_mouse_pos(&mut self, point: Point2) {
        let point: Vec<Atomic> = vec![point.x.into(), point.y.into()];
        let point = Value::List(point);
        let id = self.id.to_string();
        let path = format!("{}/mouse_position", id);
        self.shared_state.update(&path, point).unwrap();
    }

    fn extract_mouse_pos(shelf_content: &Value) -> Point2 {
        if let Value::List(list) = shelf_content {
            let point: Vec<f32> = list
                .iter()
                .filter_map(|atom| match atom {
                    Atomic::Float(f) => Some(*f),
                    _ => None,
                })
                .collect();
            Point2::new(point[0], point[1])
        } else {
            panic!("sadddd");
        }
    }

    fn get_mouse_pos(&self) -> Point2 {
        let id = self.id.to_string();
        let path = format!("{}/mouse_position", id);
        let value = self.shared_state.get(&path).unwrap();
        Self::extract_mouse_pos(value)
    }

    fn get_collaborator_mice(&self) -> Vec<Point2> {
        match self.shared_state.deref() {
            Some(Value::Map(map)) => map
                .iter()
                .filter_map(|(_, val)| val.get("mouse_position"))
                .map(Self::extract_mouse_pos)
                .collect(),
            _ => panic!("couldn't find users."),
        }
    }

    fn send_shared_state_update(&mut self) {
        let shelf = &self.shared_state;
        let sv: StateVector = shelf.into();
        let data = serde_json::to_vec(&sv).unwrap();
        self.communicator.send("update", &data);
    }

    fn receive_shared_state_update(&mut self) {
        if let Some(update) = self.communicator.try_recv() {
            match update.topic.as_ref() {
                "update" => {
                    let sv: StateVector = serde_json::from_slice(&update.data).unwrap();
                    if let Some(updates) = self.shared_state.get_state_delta(&sv) {
                        let data = serde_json::to_vec(&updates).unwrap();
                        self.communicator
                            .send(&format!("diff:{}", update.sender), &data);
                    }
                }
                "diff" => {
                    let update_shelf: Shelf<Value, Temporal> =
                        serde_json::from_slice(&update.data).unwrap();
                    let shared_state = std::mem::take(&mut self.shared_state);
                    self.shared_state = shared_state.merge(update_shelf);
                }
                _ => (),
            }
        }
    }
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
    let id = id as u8;
    let communicator = Multicast::new(id);
    let shared_state: Shelf<Value, Temporal> = Shelf::from_template(
        json!({ id.to_string(): {
        "mouse_position": [0.0, 0.0],
    }  }),
        id.to_string(),
    );

    Model {
        id,
        _window,
        effects: vec![],
        shared_state,
        communicator,
        update_frequency: 300,
        last_update_sent: 0,
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
    let now = update.since_start.as_millis();
    model.receive_shared_state_update();
    if now - model.last_update_sent > model.update_frequency as u128 {
        model.send_shared_state_update();
        model.last_update_sent = now;
    }
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
            let data = serde_json::to_string(&model.shared_state.clone()).unwrap();
            println!("{data}");
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
