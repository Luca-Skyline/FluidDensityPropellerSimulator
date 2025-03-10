use bevy::prelude::*;
use bevy::render::mesh::{shape, Mesh};// Import shapes correctly
use rand::Rng;
use nalgebra::Vector3;
use csv::Writer;
use serde::Serialize;
use std::error::Error;
use std::sync::Mutex;
use lazy_static::lazy_static;


#[derive(Component)]
struct Particle {
    velocity: Vec3,
    mass: f32,
}

#[derive(Component)]
struct Propeller {
    rotation_z: f32,
    pitch: f32,
    angular_v: f32,
    old_rotation_z: f32,
    mass: f32,
    total_vertical_impulse: f32,
}

lazy_static! {
    static ref COUNTER: Mutex<u32> = Mutex::new(0);
    static ref TIME_ELAPSED: Mutex<f32> = Mutex::new(0.0);
    static ref TRIAL: Mutex<u32> = Mutex::new(0);
    static ref DATA_ROW: Mutex<Vec<f32>> = Mutex::new(Vec::new());
}




const BOUNDING_BOX_SIZE: f32 = 10.2;
const START_PROP_VELOCITY: f32 = 20.0; //giving the propeller a small start speed prevents its velocity from exploding under constant power
const THETA_PITCH: f32 = 5.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup, spawn_particles))
        .add_systems(Update, (controller, move_particles, wall_collisions, compare_particles, draw_boundary_cube, update_rectangle_rotation, blade_collisions))
        .run();
}

// Setup camera and lighting
fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {

    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-10.0, 12.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(0.0, 10.0, 0.0),
        ..default()
    });

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Box::new(4.0, 1.0, 0.05))), // Length = 4, Width = 1, Thin height
            material: materials.add(StandardMaterial {
                base_color: Color::rgb(0.0, 0.0, 1.0), // Blue color
                ..default()
            }),
            transform: Transform {
                translation: Vec3::new(2.0, 0.0, 0.0), // Start at cube center, extend outward
                rotation: Quat::IDENTITY, // Identity rotation for now
                ..default()
            },
            ..default()
        },
        Propeller { rotation_z: 0.0, pitch: 45.0, angular_v: 20.0, old_rotation_z: 0.0, mass: 5.0, total_vertical_impulse: 0.0 }, // Custom component to track rotation
    ));
}

fn controller(mut prop_query: Query<(&mut Transform, &mut Propeller)>, mut part_query: Query<(&mut Transform, &mut Particle), Without<Propeller>>, time: Res<Time>,){
    let mut total_time = TIME_ELAPSED.lock().unwrap();
    *total_time += time.delta_seconds();
    
    if(*total_time >= 10.0){

        *total_time = 0.0;
        let mut trial = TRIAL.lock().unwrap();
        *trial += 1;

        let mut data = DATA_ROW.lock().unwrap();
        for (mut transform, mut prop) in prop_query.iter_mut(){
            data.push(prop.total_vertical_impulse);
            prop.rotation_z = 0.0;
            prop.old_rotation_z = 0.0;
            prop.angular_v = START_PROP_VELOCITY;
            prop.total_vertical_impulse = 0.0;


            if(*trial == 8){
                let mut avg = 0.0;
                for datum in data.iter(){
                    avg += datum;
                }
                avg = avg / 8.0;
                data.push(avg);
                if let Err(err) = append_to_csv("output.csv", &data) {
                    eprintln!("Error writing CSV: {}", err);
                } else {
                    println!("Successful writing to CSV");
                }
                prop.pitch += THETA_PITCH;
                *data = Vec::new();
                *trial = 0;

                //if prop.pitch == 85.0, quit program
                if prop.pitch == 85.0{
                    std::process::exit(0);
                }
            }
           
        }

        let mut rng = rand::thread_rng();
        for (mut transform, mut part) in part_query.iter_mut(){
            transform.translation = Vec3::new(rng.gen_range(-5.0..5.0),rng.gen_range(-5.0..5.0),rng.gen_range(-5.0..5.0));
            part.velocity = Vec3::new(rng.gen_range(-1.0..1.0),rng.gen_range(-1.0..1.0),rng.gen_range(-1.0..1.0));
        }

    }
 
}

fn append_to_csv(file_path: &str, data: &Vec<f32>) -> Result<(), Box<dyn Error>> {
    let mut wtr = Writer::from_writer(std::fs::OpenOptions::new().append(true).open(file_path)?);

    let string_data: Vec<String> = data.iter().map(|&num| num.to_string()).collect();
    let string_refs: Vec<&str> = string_data.iter().map(|s| s.as_str()).collect();

    // Write the vector as a single CSV record
    wtr.write_record(&string_refs)?;

    wtr.flush()?;
    Ok(())
}

// fn write_csv_no_header(file_path: &str, data: &Vec<f32>) -> Result<(), Box<dyn Error>> {
//     let mut wtr = Writer::from_path(file_path)?;

//     let string_data: Vec<String> = data.iter().map(|&num| num.to_string()).collect();
//     let string_refs: Vec<&str> = string_data.iter().map(|s| s.as_str()).collect();

//     // Write the vector as a single CSV record
//     wtr.write_record(&string_refs)?;

//     wtr.flush()?;
//     Ok(())
// }


// Spawn particles with visible 3D spheres
fn spawn_particles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let sphere_mesh = Mesh::try_from(shape::Icosphere { radius: 0.1, subdivisions: 4 })
        .expect("Failed to create sphere mesh");
    let sphere_handle = meshes.add(sphere_mesh);

    let material_handle = materials.add(StandardMaterial {
        base_color: Color::rgb(1.0, 0.0, 0.0), // Red particles
        ..default()
    });

    let mut rng = rand::thread_rng();
    for _ in 0..400 {
        commands.spawn((
            PbrBundle {
                mesh: sphere_handle.clone(),
                material: material_handle.clone(),
                transform: Transform::from_xyz(
                    rng.gen_range(-5.0..5.0),
                    rng.gen_range(-5.0..5.0),
                    rng.gen_range(-5.0..5.0),
                ),
                ..default()
            },
            Particle {
                //velocity: Vec3::new(0.0, 0.0, 0.0),
                velocity: Vec3::new(
                    rng.gen_range(-1.0..1.0),
                    rng.gen_range(-1.0..1.0),
                    rng.gen_range(-1.0..1.0),
                ),
                mass: 5.0,
            },
        ));
    }
}


// Update particle movement each frame
fn move_particles(mut query: Query<(&mut Transform, &Particle)>, time: Res<Time>) {
    for (mut transform, particle) in query.iter_mut() {
        transform.translation += particle.velocity * time.delta_seconds();
    }
}

fn distance_between(a: &Transform, b: &Transform) -> f32 {
    a.translation.distance(b.translation)
}

// Bounce particles off the walls
fn wall_collisions(mut query: Query<(&mut Transform, &mut Particle)>) {
    for (mut transform, mut particle) in query.iter_mut() {
        for i in 0..3 {
            if transform.translation[i].abs() > 5.0 {
                transform.translation[i] = (transform.translation[i] / transform.translation[i].abs()) * 5.0;
                //println!("{}", transform.translation[i].to_string());
                particle.velocity[i] *= -1.0;
            }
        }
    }
}

fn compare_particles(mut query: Query<(&mut Transform, &mut Particle)>, time: Res<Time>) {
    let mut entities: Vec<(Mut<Transform>, Mut<Particle>)> = query.iter_mut().collect();

    for i in 0..entities.len() {
        // Split the slice to get separate mutable references
        let (left, right) = entities.split_at_mut(i + 1);

        for j in 0..right.len() {
            let (transform_a, particle_a) = &mut left[i];
            let (transform_b, particle_b) = &mut right[j];


            if distance_between(transform_a, transform_b) <= 0.2 {
                //println!("Collision!");
                transform_a.translation += -1.0 * particle_a.velocity * time.delta_seconds();
                transform_b.translation += -1.0 * particle_b.velocity * time.delta_seconds();
                let v_temp = particle_a.velocity;
                particle_a.velocity = particle_b.velocity;
                particle_b.velocity = v_temp;
            }
        }
    }
}

fn blade_collisions(mut commands: Commands, mut propeller_query: Query<(&Transform, &mut Propeller)>, // Immutable
mut particle_query: Query<(Entity, &mut Transform, &mut Particle), Without<Propeller>>, // Mutable
time: Res<Time>, mut gizmos: Gizmos
) {
    for (prop_transform, mut propeller) in propeller_query.iter_mut() {
        for (particle_entity, mut part_transform, mut particle) in particle_query.iter_mut() {
            // Perform comparison and update particles
            if(part_transform.translation[1].abs() < 0.5*(propeller.pitch.to_radians().sin())){
                let temp_transform = Transform::default(); //(0, 0, 0)
                if(distance_between(&part_transform, &temp_transform) < 4.0){
                    let mut theta = (part_transform.translation[0]/part_transform.translation[2]).atan();
                    //println!("{}", theta.to_string());
                    if part_transform.translation[2] < 0.0{
                        theta += 3.14159265;
                    }
                    else if theta < 0.0{
                        theta += 6.28315307
                    }

                    let angle_modifier = (propeller.pitch.to_radians().cos()/(2.0*distance_between(&part_transform, &temp_transform))).atan();

                    if theta < propeller.rotation_z.to_radians() + angle_modifier && theta > propeller.old_rotation_z.to_radians() - angle_modifier{
                        let unit_parallel = Vector3::new(propeller.rotation_z.to_radians().sin(),0.0, propeller.rotation_z.to_radians().cos());

                        //unit tilt
                        let down_value = -(propeller.pitch.to_radians().sin()); 
                        let out_value = (1.0-(&down_value*&down_value)).sqrt(); //keep unit
                        let unit_tilt = Vector3::new((propeller.rotation_z - 90.0).to_radians().sin() * out_value, down_value, (propeller.rotation_z - 90.0).to_radians().cos() * out_value);
                       
                        let unit_normal = unit_parallel.cross(&unit_tilt);

                        let particle_distance = distance_between(&part_transform, &temp_transform);
                        let propeller_speed = propeller.angular_v * particle_distance / 360.0;
                        let propeller_velocity = propeller_speed * Vector3::new((propeller.rotation_z + 90.0).to_radians().sin(), 0.0, (propeller.rotation_z + 90.0).to_radians().cos());
                        let net_velocity = Vector3::new(particle.velocity[0], particle.velocity[1], particle.velocity[2]) - propeller_velocity;
                        let scalar = net_velocity.dot(&unit_normal);
                        let mut impulse_vector = (2.0*propeller.mass*particle.mass*scalar*&unit_normal)/(propeller.mass + particle.mass);

                        propeller.total_vertical_impulse += impulse_vector[1];

                        gizmos.line(Vec3::new(0.0,0.0,0.0), Vec3::new(impulse_vector[0], impulse_vector[1], impulse_vector[2]), Color::RED);

                        impulse_vector[1] = 0.0;

                        let moment_arm = Vector3::new(part_transform.translation[0], 0.0, part_transform.translation[2]);
                        let angular_impulse = moment_arm.cross(&impulse_vector);

                        let unit_vertial = Vector3::new(0.0, -1.0, 0.0);
                        let angular_impulse_mag = angular_impulse.dot(&unit_vertial);

                        let moi = (1.0/3.0) * propeller.mass * 16.0;

                        let delta_angular_v = -angular_impulse_mag / moi;

                        propeller.angular_v += delta_angular_v;                        
                        
                        gizmos.line(Vec3::new(0.0,0.0,0.0), Vec3::new(moment_arm[0], moment_arm[1], moment_arm[2]), Color::WHITE);

                        let mut rng = rand::thread_rng();
                        part_transform.translation = Vec3::new(rng.gen_range(-5.0..5.0),rng.gen_range(-5.0..5.0),rng.gen_range(-5.0..5.0));
                        

                        //commands.entity(particle_entity).despawn();
                        //let output = format!("Collision. Propeller angle: {}, particle angle: {}, old propeller angle: {}, vector: {}", propeller.rotation_z.to_string(),theta.to_string(), propeller.old_rotation_z.to_string(), propeller_velocity.to_string());
                        //("{}", delta_angular_v.to_string());
                    }
                }
            }
        }
    }
}

fn update_rectangle_rotation(mut query: Query<(&mut Propeller, &mut Transform)>, time: Res<Time>) {
    for (mut rect, mut transform) in query.iter_mut() {
        if(rect.rotation_z >= 360.0){
            rect.rotation_z -= 360.0;
        }
        let moi = (1.0/3.0) * rect.mass * 16.0;
        rect.angular_v += (50000.0*time.delta_seconds())/(moi * rect.angular_v.to_radians()).to_degrees();
        //println!("{}", rect.angular_v.to_string());
        //println!("{}", rect.rotation_z.to_string());
        rect.old_rotation_z = rect.rotation_z;
        rect.rotation_z += rect.angular_v * time.delta_seconds();
        let rotation_z = Quat::from_rotation_y(rect.rotation_z.to_radians() + (3.14159265/2.0));
        let rotation_pitch = Quat::from_rotation_x((90.0 - rect.pitch).to_radians());

        // Rotate around cube center by first moving it to (0,0,0), rotating, and moving back
        let pivot = Vec3::new(-2.0, 0.0, 0.0); // Move back by half its length before rotating
        transform.translation = rotation_z * pivot + Vec3::ZERO;
        transform.rotation = rotation_z * rotation_pitch;
    }
}

fn draw_boundary_cube(mut gizmos: Gizmos) {
    let half_size = BOUNDING_BOX_SIZE / 2.0;

    let corners = [
        Vec3::new(-half_size, -half_size, -half_size),
        Vec3::new(half_size, -half_size, -half_size),
        Vec3::new(half_size, half_size, -half_size),
        Vec3::new(-half_size, half_size, -half_size),
        Vec3::new(-half_size, -half_size, half_size),
        Vec3::new(half_size, -half_size, half_size),
        Vec3::new(half_size, half_size, half_size),
        Vec3::new(-half_size, half_size, half_size),
    ];

    let edges = [
        (0, 1), (1, 2), (2, 3), (3, 0), // Bottom square
        (4, 5), (5, 6), (6, 7), (7, 4), // Top square
        (0, 4), (1, 5), (2, 6), (3, 7), // Vertical edges
    ];

    for &(start, end) in &edges {
        gizmos.line(corners[start], corners[end], Color::WHITE);
    }
}
