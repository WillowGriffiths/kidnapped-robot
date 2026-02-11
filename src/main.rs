use bevy::{
    dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin, FrameTimeGraphConfig},
    prelude::*,
    text::FontSmoothing,
};
use rand::RngExt;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Kidnapped Robot!".into(),
                    resolution: (1000, 1000).into(),
                    ..default()
                }),
                ..default()
            }),
            FpsOverlayPlugin {
                config: FpsOverlayConfig {
                    text_config: TextFont {
                        font_size: 24.0,
                        font: default(),
                        font_smoothing: FontSmoothing::default(),
                        ..default()
                    },
                    text_color: Color::oklch(1.0, 0.0, 0.0),
                    refresh_interval: core::time::Duration::from_millis(1000),
                    enabled: true,
                    frame_time_graph_config: FrameTimeGraphConfig {
                        enabled: true,
                        min_fps: 30.0,
                        target_fps: 120.0,
                    },
                },
            },
        ))
        .add_systems(Startup, startup)
        .add_systems(Update, draw_particles)
        .add_systems(Update, update_particles)
        .run();
}

struct Particle(Vec2);

#[derive(Component)]
struct Particles(Vec<Particle>);

fn startup(mut commands: Commands) {
    let particle_count = 1000;
    let mut particles: Vec<Particle> = Vec::with_capacity(particle_count);
    let mut rng = rand::rng();

    for _ in 0..particle_count {
        particles.push(Particle(
            vec2(rng.random(), rng.random()) * 2.0 - vec2(1.0, 1.0),
        ));
    }

    commands.spawn(Particles(particles));
    commands.spawn(Camera2d);
}

fn draw_particles(mut gizmos: Gizmos, particles: Single<&Particles>) {
    for Particle(pos) in &particles.0 {
        gizmos.circle_2d(
            Isometry2d::from_translation(pos * 500.0),
            0.5,
            Color::oklch(0.9, 0.0, 0.0),
        );
    }
}

fn update_particles(time: Res<Time>, mut particles: Single<&mut Particles>) {
    for particle in &mut particles.0 {
        particle.0 += particle.0.rotate(Vec2::Y) * time.delta_secs() * 0.1;
    }
}
