use std::{f32::consts::PI, time::Duration};

use bevy::{
    dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin, FrameTimeGraphConfig},
    prelude::*,
    sprite::Anchor,
    text::FontSmoothing,
};
use bevy_rapier2d::{na::ComplexField, prelude::*};
use rand::prelude::Distribution;
use statrs::distribution;

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
            RapierPhysicsPlugin::<NoUserData>::default(),
            RapierDebugRenderPlugin::default(),
        ))
        .add_systems(Startup, startup)
        .add_systems(Update, draw_particles)
        .add_systems(Update, draw_timer)
        .add_systems(FixedUpdate, update_particles)
        .add_systems(FixedUpdate, update_robot)
        .add_systems(FixedUpdate, do_raycast)
        .run();
}

#[derive(Clone, Copy)]
struct Particle(Vec2, f32);

#[derive(Component)]
struct Particles(Vec<Particle>);

#[derive(Component)]
struct TargetVelocity(f32, f32);

#[derive(Component)]
struct Robot;

#[derive(Component)]
struct RaycastTimer(Timer);

#[derive(Component)]
struct LastRaycast(f32);

#[derive(Component)]
struct TimerText;

#[derive(Resource)]
struct Noise {
    velocity: distribution::Normal,
    rotation: distribution::Normal,
    artificial: distribution::Normal,
}

#[derive(Resource)]
struct WeightDisplay(Vec<f32>);

const PARTICLE_COUNT: usize = 10_000;
const START_ROTATION: f32 = PI / 2.0;
const WEIGHT_DISPLAY_RESOLUTION: usize = 100;

// Standard deviation for the observation likelihood (in world units).
// Tune this to control how sharply the filter penalises distance error.
const RAYCAST_SIGMA: f32 = 40.0;

fn startup(mut commands: Commands) {
    let mut particles: Vec<Particle> = Vec::with_capacity(PARTICLE_COUNT);

    for _ in 0..PARTICLE_COUNT {
        particles.push(Particle(
            (vec2(rand::random::<f32>(), rand::random::<f32>()) - vec2(0.5, 0.5)) * 1000.0,
            rand::random::<f32>() * PI * 2.0,
        ));
    }

    commands.spawn(Particles(particles));

    commands
        .spawn(RigidBody::Fixed)
        .insert(Collider::cuboid(100.0, 100.0))
        .insert(
            Transform::from_xyz(100.0, 250.0, 0.0)
                * Transform::from_rotation(Quat::from_rotation_z(1.0)),
        );

    commands
        .spawn(RigidBody::Fixed)
        .insert(Collider::ball(40.0))
        .insert(
            Transform::from_xyz(-100.0, -100.0, 0.0)
                * Transform::from_rotation(Quat::from_rotation_z(2.0)),
        );

    commands
        .spawn(RigidBody::Fixed)
        .insert(Collider::cuboid(10.0, 500.0))
        .insert(Transform::from_xyz(-500.0, 0.0, 0.0));

    commands
        .spawn(RigidBody::Fixed)
        .insert(Collider::cuboid(10.0, 500.0))
        .insert(Transform::from_xyz(500.0, 0.0, 0.0));

    commands
        .spawn(RigidBody::Fixed)
        .insert(Collider::cuboid(500.0, 10.0))
        .insert(Transform::from_xyz(0.0, -500.0, 0.0));

    commands
        .spawn(RigidBody::Fixed)
        .insert(Collider::cuboid(500.0, 10.0))
        .insert(Transform::from_xyz(0.0, 500.0, 0.0));

    commands
        .spawn(Robot)
        .insert(RigidBody::Dynamic)
        .insert(Collider::ball(50.0))
        .insert(GravityScale(0.0))
        .insert(Velocity::zero())
        .insert(Transform::from_rotation(Quat::from_rotation_z(
            START_ROTATION,
        )))
        .insert(TargetVelocity(0.0, 0.0))
        .insert(RaycastTimer(Timer::new(
            Duration::from_millis(750),
            TimerMode::Repeating,
        )))
        .insert(LastRaycast(0.0));

    commands.spawn(Camera2d);

    commands
        .spawn(Text2d::new(""))
        .insert(TimerText)
        .insert(TextLayout::new_with_justify(Justify::Left))
        .insert(Anchor::TOP_LEFT)
        .insert(Transform::from_xyz(-480.0, 410.0, 0.0));

    commands.insert_resource(Noise {
        velocity: distribution::Normal::new(1.0, 0.3).unwrap(),
        rotation: distribution::Normal::new(1.0, 0.3).unwrap(),
        artificial: distribution::Normal::new(1.0, 1.0).unwrap(),
    });
}

fn draw_particles(mut gizmos: Gizmos, particles: Single<&Particles>) {
    for Particle(pos, _t) in &particles.0 {
        gizmos.circle_2d(
            Isometry2d::from_translation(*pos),
            0.5,
            Color::oklch(0.9, 0.0, 0.0),
        );
    }
}

fn draw_timer(
    robot: Single<&mut RaycastTimer, With<Robot>>,
    mut text: Single<&mut Text2d, With<TimerText>>,
) {
    let time = robot.0.remaining_secs();
    text.0 = format!("Time to raycast: {time:.3}");
}

fn update_particles(
    time: Res<Time>,
    noise: Res<Noise>,
    mut particles: Single<&mut Particles>,
    robot: Single<&mut TargetVelocity, With<Robot>>,
) {
    let mut rng = rand::thread_rng();
    for particle in &mut particles.0 {
        particle.1 += robot.1 * time.delta_secs();
        particle.0 += Vec2::from_angle(particle.1)
            * robot.0
            * time.delta_secs()
            * noise.artificial.sample(&mut rng) as f32;
    }
}

const ROBOT_SPEED: f32 = 100.0;
const ROBOT_ANGULAR_SPEED: f32 = 0.5 * PI;
const ROTATE_THRESHOLD: f32 = 200.0;

fn update_robot(
    time: Res<Time>,
    noise: Res<Noise>,
    robot: Single<(&Transform, &mut Velocity, &mut TargetVelocity, &LastRaycast), With<Robot>>,
) {
    if time.elapsed_secs() < 3.0 {
        return;
    }
    let (transform, mut velocity, mut target_velocity, last_raycast) = robot.into_inner();

    target_velocity.0 = if last_raycast.0 > ROTATE_THRESHOLD {
        ROBOT_SPEED
    } else {
        0.0
    };
    let direction = (transform.rotation * Vec3::X).xy() * target_velocity.0;

    target_velocity.1 = if last_raycast.0 > ROTATE_THRESHOLD {
        0.0
    } else {
        ROBOT_ANGULAR_SPEED
    };

    let mut rng = rand::thread_rng();
    let vel_noise = vec2(
        noise.velocity.sample(&mut rng) as f32,
        noise.velocity.sample(&mut rng) as f32,
    );
    let rot_noise = noise.rotation.sample(&mut rng) as f32;

    velocity.linvel = direction * vel_noise;
    velocity.angvel = target_velocity.1 * rot_noise;
}

fn observation_likelihood(measured: f32, expected: f32) -> f32 {
    let diff = measured - expected;
    (-(diff * diff) / (2.0 * RAYCAST_SIGMA * RAYCAST_SIGMA)).exp()
}

fn do_raycast(
    time: Res<Time>,
    robot: Single<(&mut RaycastTimer, &Transform, &mut LastRaycast), With<Robot>>,
    ctx: ReadRapierContext,
    mut particles: Single<&mut Particles>,
) {
    let (mut timer, transform, mut last_raycast) = robot.into_inner();
    timer.0.tick(time.delta());

    if !timer.0.just_finished() {
        return;
    }

    let ctx = ctx.single().unwrap();

    let robot_dir = (transform.rotation * Vec3::X).xy();
    let raycast = ctx.cast_ray_and_get_normal(
        transform.translation.xy(),
        robot_dir,
        2000.0,
        false,
        QueryFilter::exclude_dynamic(),
    );

    let Some((_entity, intersection)) = raycast else {
        return;
    };

    let measured_dist = intersection.time_of_impact;
    last_raycast.0 = measured_dist;

    info!(
        "raycast hit: dist={:.1}, normal={:?}",
        measured_dist, intersection.normal
    );

    let mut weights = [0.0_f32; PARTICLE_COUNT];

    for (i, Particle(pos, angle)) in particles.0.iter().enumerate() {
        let particle_dir = Vec2::from_angle(*angle);

        let simulated = ctx.cast_ray(
            *pos,
            particle_dir,
            2000.0,
            false,
            QueryFilter::exclude_dynamic(),
        );

        let expected_dist = simulated.map_or(2000.0, |(_e, toi)| toi);
        weights[i] = observation_likelihood(measured_dist, expected_dist);
    }

    let weight_sum: f32 = weights.iter().sum();
    if weight_sum == 0.0 {
        return;
    }

    let mut cum_weights: Vec<(usize, f32)> = Vec::with_capacity(PARTICLE_COUNT);
    let mut running = 0.0_f32;
    for (i, &w) in weights.iter().enumerate() {
        running += w / weight_sum;
        cum_weights.push((i, running));
    }

    const SPREAD: usize = 1_000;

    let new_particles: Vec<Particle> = (0..(PARTICLE_COUNT - SPREAD))
        .map(|_| {
            let x = rand::random::<f32>();

            let i = cum_weights
                .partition_point(|&(_, cum)| cum < x)
                .min(PARTICLE_COUNT - 1);

            particles.0[cum_weights[i].0]
        })
        .chain((0..SPREAD).map(|_| {
            Particle(
                (vec2(rand::random::<f32>(), rand::random::<f32>()) - vec2(0.5, 0.5)) * 1000.0,
                rand::random::<f32>() * PI * 2.0,
            )
        }))
        .collect();

    particles.0 = new_particles;
}
