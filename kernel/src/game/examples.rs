//! # Game Engine Examples
//!
//! Examples demonstrating how to use the game engine systems.

use crate::game::*;

/// Example 1: Basic Physics
pub fn basic_physics_example() {
    // Create physics world
    let config = PhysicsConfig::default();
    let mut physics_world = PhysicsWorld::new(config);

    // Create a rigid body
    let transform = Transform::new(
        Vec3::new(0.0, 10.0, 0.0),
        Quaternion::identity(),
        Vec3::one(),
    );

    let body = RigidBody::new(transform, 10.0)
        .with_collider(Collider::sphere(1.0))
        .with_material(Material::rubber());

    let body_handle = physics_world.add_body(body);

    // Simulate
    for _ in 0..60 {
        physics_world.step(1.0 / 60.0);
    }

    // Query body state
    if let Some(updated_body) = physics_world.get_body(body_handle) {
        let position = updated_body.transform.position;
        println!("Body position: {:?}", position);
    }
}

/// Example 2: Particle Effects
pub fn particle_effect_example() {
    // Create fire effect
    let fire_position = Vec3::new(0.0, 0.0, 0.0);
    let mut fire_system = particle::effects::FireEffect::create(fire_position);

    // Update particles
    for _ in 0..100 {
        fire_system.update(1.0 / 60.0);
    }

    // Get active particles
    let active_count = fire_system.get_active_particles();
    println!("Active particles: {}", active_count);
}

/// Example 3: Animation System
pub fn animation_example() {
    // Create animation clip
    let mut walk_clip = AnimationClip::new(alloc::string::String::from("Walk"), 1.0);

    // Add animation track
    let hip_track = animation::AnimationTrack::new(alloc::string::String::from("Hip"));
    walk_clip.add_track(hip_track);

    // Create animation mixer
    let mixer = AnimationMixer::new();

    // Create animation layer
    let mut layer = animation::AnimationLayer::new(alloc::string::String::from("Base"));

    // Add animation state
    let state = animation::AnimationState::new(walk_clip);
    layer.add_state(state);

    // Update animation
    layer.update(1.0 / 60.0);
}

/// Example 4: Scene Management
pub fn scene_management_example() {
    // Create scene graph
    let mut scene = SceneGraph::new();

    // Create node
    let node = SceneNode::new(
        alloc::string::String::from("Player"),
        NodeType::Mesh,
    )
    .with_transform(Transform::new(
        Vec3::new(0.0, 0.0, 0.0),
        Quaternion::identity(),
        Vec3::one(),
    ));

    let node_id = scene.add_node(node);

    // Set parent
    scene.set_parent(node_id, scene.root);

    // Update transforms
    scene.update_transforms(node_id);

    // Query nodes
    if let Some(found_node) = scene.find_node_by_name("Player") {
        println!("Found node: {}", found_node.name);
    }
}

/// Example 5: 3D Audio
pub fn audio_example() {
    // Create audio engine
    let mut audio_engine = AudioEngine::new(32);

    // Create listener
    let listener = AudioListener::new();
    audio_engine.set_listener(listener);

    // Create sound source
    let source = SoundSource::new(SoundId::new())
        .with_position(Vec3::new(10.0, 0.0, 0.0))
        .with_volume(0.8);

    audio_engine.add_source(source);

    // Calculate audio parameters
    let sources: Vec<_> = audio_engine
        .sources
        .values()
        .collect();

    for source in sources {
        let params = audio_engine.calculate_source_parameters(source);
        println!("Source volume: {}", params.volume);
    }
}

/// Example 6: Input System
pub fn input_system_example() {
    // Create input manager
    let mut input_manager = InputManager::new();

    // Create input context
    let mut context = InputContext::new(alloc::string::String::from("Game"));

    // Create input actions
    let mut move_action = InputAction::new(alloc::string::String::from("Move"));
    move_action = move_action
        .with_binding(InputBinding::Key(KeyCode::W))
        .with_binding(InputBinding::GamepadAxis(GamepadAxis::LeftStickY));

    context.add_action(move_action);
    input_manager.add_context(context);

    // Set active context
    input_manager.set_active_context(0);

    // Check input
    let _dt = 1.0 / 60.0;
    input_manager.update(_dt);

    if input_manager.is_action_pressed("Move") {
        let value = input_manager.get_action_value("Move");
        println!("Move value: {}", value);
    }
}

/// Example 7: Complete Game Loop
pub fn game_loop_example() {
    // Initialize systems
    let config = GameEngineConfig::default();
    let mut physics_world = PhysicsWorld::new(PhysicsConfig::default());
    let mut particle_systems: Vec<ParticleSystem> = Vec::new();
    let mut animation_mixer = AnimationMixer::new();
    let mut scene = SceneGraph::new();
    let mut audio_engine = AudioEngine::new(config.max_sounds);
    let mut input_manager = InputManager::new();

    // Set up audio listener
    let listener = AudioListener::new();
    audio_engine.set_listener(listener);

    // Game loop
    let target_frame_time = 1.0 / config.target_fps as f64;
    let mut accumulated_time = 0.0;
    let mut last_time = 0.0;

    loop {
        // Calculate delta time
        let current_time = 0.0; // Get actual time
        let frame_time = (current_time - last_time) as f64;
        last_time = current_time;

        // Fixed timestep update
        accumulated_time += frame_time;

        while accumulated_time >= target_frame_time {
            // Update input
            input_manager.update(target_frame_time as Float);

            // Update physics
            physics_world.step(target_frame_time as Float);

            // Update particles
            for system in &mut particle_systems {
                system.update(target_frame_time as Float);
            }

            // Update animations
            animation_mixer.update(target_frame_time as Float);

            // Update scene
            // scene.update(...);

            // Update audio
            audio_engine.update(target_frame_time as Float);

            accumulated_time -= target_frame_time;
        }

        // Render
        // render(...);

        // Break for demo
        break;
    }
}
