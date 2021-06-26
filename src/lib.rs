#![warn(clippy::pedantic, clippy::nursery)]

// Heavily based upon https://github.com/bevyengine/bevy/blob/latest/examples/game/breakout.rs
// Font from https://www.fontspace.com/paul-font-f22964

use bevy::{
    core::Time,
    ecs::system::IntoSystem,
    input::Input,
    math::{Rect, Vec2, Vec3},
    prelude::*,
    sprite::{
        collide_aabb::{collide, Collision},
        ColorMaterial, Sprite,
    },
    text::{Text, TextSection, TextStyle},
    ui::{PositionType, Style, Val},
    window::WindowDescriptor,
    DefaultPlugins,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

const DIM: f32 = 500.;
const PADDLE_HEIGHT: f32 = 100.;
const WALL_THICKNESS: f32 = 10.0;
const BOUND: f32 = 2. * PADDLE_HEIGHT - 0.5 * WALL_THICKNESS;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn run() {
    let mut app = App::build();
    app.add_plugins(DefaultPlugins)
        .insert_resource(WindowDescriptor {
            title: "Pong!".to_string(),
            width: DIM,
            height: DIM,
            resizable: false,
            vsync: true,
            ..Default::default()
        })
        .insert_resource(ScoreBoard::default())
        .insert_resource(ClearColor(Color::rgb(0.9, 0.9, 0.9)))
        .insert_resource(Pauser { paused: false })
        .add_startup_system(setup.system())
        .add_system(paddle_movement_system.system())
        .add_system(ball_collision_system.system())
        .add_system(ball_movement_system.system())
        .add_system(score.system())
        .add_system(pause_system.system())
        .add_event::<ScoreEvent>();

    // when building for Web, use WebGL2 rendering
    #[cfg(target_arch = "wasm32")]
    app.add_plugin(bevy_webgl2::WebGL2Plugin);

    app.run();
}

struct Pauser {
    paused: bool,
}

enum Collider {
    Paddle,
    Wall,
    Left,
    Right,
}

enum ScoreEvent {
    Left,
    Right,
}

#[derive(Default)]
struct ScoreBoard {
    left: usize,
    right: usize,
}

impl core::fmt::Display for ScoreBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.left, self.right)
    }
}

#[derive(Debug)]
struct Paddle {
    team: usize,
    speed: f32,
}

struct Ball {
    velocity: Vec3,
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
    scoreboard: Res<ScoreBoard>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());

    for (team, x) in (&[-DIM + 10., DIM - 10.]).iter().enumerate() {
        commands
            .spawn_bundle(SpriteBundle {
                material: materials.add(Color::rgb(0.5, 0.5, 1.0).into()),
                transform: Transform::from_xyz(*x, 0., 1.0),
                sprite: Sprite::new(Vec2::new(WALL_THICKNESS, PADDLE_HEIGHT)),
                ..Default::default()
            })
            .insert(Paddle { team, speed: 500. })
            .insert(Collider::Paddle);
    }

    commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(Color::rgb(0.2, 0.7, 0.0).into()),
            transform: Transform::from_xyz(0., 0., 0.0),
            sprite: Sprite::new(Vec2::new(50.0, 50.0)),
            ..Default::default()
        })
        .insert(Ball {
            velocity: Vec3::new(500., 50., 0.),
        });

    let bounds = Vec2::new(DIM, DIM);

    // left
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform::from_xyz(-bounds.x, 0.0, 0.0),
            sprite: Sprite::new(Vec2::new(WALL_THICKNESS, bounds.y + WALL_THICKNESS)),
            ..Default::default()
        })
        .insert(Collider::Left);
    // right
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform::from_xyz(bounds.x, 0.0, 0.0),
            sprite: Sprite::new(Vec2::new(WALL_THICKNESS, bounds.y + WALL_THICKNESS)),
            ..Default::default()
        })
        .insert(Collider::Right);

    // bottom
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform::from_xyz(0.0, -bounds.y / 2.0, 0.0),
            sprite: Sprite::new(Vec2::new(bounds.x * 2., WALL_THICKNESS)),
            ..Default::default()
        })
        .insert(Collider::Wall);
    // top
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform::from_xyz(0.0, bounds.y / 2.0, 0.0),
            sprite: Sprite::new(Vec2::new(bounds.x * 2., WALL_THICKNESS)),
            ..Default::default()
        })
        .insert(Collider::Wall);

    commands.spawn_bundle(TextBundle {
        text: Text {
            sections: vec![TextSection {
                value: scoreboard.to_string(),
                style: TextStyle {
                    font_size: 40.0,
                    color: Color::rgb(1.0, 1.0, 1.0),
                    font: asset_server.load("Paul-le1V.ttf"),
                },
            }],
            ..Default::default()
        },
        style: Style {
            position_type: PositionType::Absolute,
            position: Rect {
                top: Val::Px(5.),
                left: Val::Percent(50.),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    });
}

fn paddle_movement_system(
    time: Res<Time>,
    pauser: Res<Pauser>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&Paddle, &mut Transform)>,
) {
    if !pauser.paused {
        for (paddle, mut transform) in query.iter_mut() {
            let mut direction = 0.0;
            if paddle.team == 0 {
                direction += keyboard_input.pressed(KeyCode::W) as u8 as f32;
                direction -= keyboard_input.pressed(KeyCode::S) as u8 as f32;
            } else {
                direction += keyboard_input.pressed(KeyCode::Up) as u8 as f32;
                direction -= keyboard_input.pressed(KeyCode::Down) as u8 as f32;
            }

            let translation = &mut transform.translation;

            // move the paddle vertically
            translation.y += time.delta_seconds() * direction * paddle.speed;
            // bound the paddle within the walls
            translation.y = translation.y.min(BOUND).max(-BOUND);
        }
    }
}

fn pause_system(keyboard_input: Res<Input<KeyCode>>, mut pauser: ResMut<Pauser>) {
    if keyboard_input.just_released(KeyCode::P) {
        pauser.paused = !pauser.paused;
    }
}

fn ball_movement_system(
    time: Res<Time>,
    pauser: Res<Pauser>,
    mut ball_query: Query<(&Ball, &mut Transform)>,
) {
    if !pauser.paused {
        // clamp the timestep to stop the ball from escaping when the game starts
        let delta_seconds = f32::min(0.2, time.delta_seconds());

        if let Ok((ball, mut transform)) = ball_query.single_mut() {
            transform.translation += ball.velocity * delta_seconds;
        }
    }
}

fn ball_collision_system(
    mut ev_score: EventWriter<ScoreEvent>,
    mut ball_query: Query<(&mut Ball, &Transform, &Sprite)>,
    collider_query: Query<(&Transform, &Sprite, &Collider)>,
) {
    if let Ok((mut ball, ball_transform, sprite)) = ball_query.single_mut() {
        let ball_size = sprite.size;
        let velocity = &mut ball.velocity;

        // check collision with walls and paddles
        for (transform, sprite, collider) in collider_query.iter() {
            let collision = collide(
                ball_transform.translation,
                ball_size,
                transform.translation,
                sprite.size,
            );
            if let Some(collision) = collision {
                match collider {
                    Collider::Wall => {
                        // reflect the ball when it collides
                        let mut reflect_y = false;

                        // only reflect if the ball's velocity is going in the opposite direction of the
                        // collision
                        match collision {
                            Collision::Top => reflect_y = velocity.y < 0.0,
                            Collision::Bottom => reflect_y = velocity.y > 0.0,
                            _ => {}
                        }

                        // reflect velocity on the y-axis if we hit something on the y-axis
                        if reflect_y {
                            velocity.y = -velocity.y;
                            velocity.y += 5.;
                        }
                    }
                    Collider::Paddle => {
                        // reflect the ball when it collides
                        let mut reflect_x = false;

                        // only reflect if the ball's velocity is going in the opposite direction of the
                        // collision
                        match collision {
                            Collision::Left => reflect_x = velocity.x > 0.0,
                            Collision::Right => reflect_x = velocity.x < 0.0,
                            _ => {}
                        }

                        // reflect velocity on the x-axis if we hit something on the x-axis
                        if reflect_x {
                            velocity.x = -velocity.x;
                            velocity.x += 5.;
                        }

                        let distance_to_mid =
                            ball_transform.translation.y - transform.translation.y;

                        velocity.y = distance_to_mid * 10.;
                    }
                    Collider::Left => ev_score.send(ScoreEvent::Left),
                    Collider::Right => ev_score.send(ScoreEvent::Right),
                }
            }
        }
    }
}

fn score(
    mut text: Query<&mut Text>,
    mut scoreboard: ResMut<ScoreBoard>,
    mut ev_score: EventReader<ScoreEvent>,
    mut ball: Query<(&mut Ball, &mut Transform)>,
) {
    if let Ok((mut ball, mut transform)) = ball.single_mut() {
        if let Ok(mut text) = text.single_mut() {
            for event in ev_score.iter() {
                match event {
                    ScoreEvent::Left => {
                        scoreboard.right += 1;
                        transform.translation = Vec3::new(0., 0., 0.0);
                        ball.velocity = Vec3::new(500., 50., 0.);
                    }
                    ScoreEvent::Right => {
                        scoreboard.left += 1;
                        transform.translation = Vec3::new(0., 0., 0.0);
                        ball.velocity = Vec3::new(-500., 50., 0.);
                    }
                }
            }
            text.sections[0].value = scoreboard.to_string();
        }
    }
}
