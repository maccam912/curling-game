use bevy::prelude::*;
use crate::components::{MainCamera, ReflectionCamera};
use crate::constants::{SHEET_LENGTH, SHEET_WIDTH};

/// Updates the reflection camera to mirror the main camera across the ice plane (Z=0).
pub fn update_reflection_camera(
    main_camera: Query<&Transform, (With<MainCamera>, Without<ReflectionCamera>)>,
    mut reflection_camera: Query<&mut Transform, (With<ReflectionCamera>, Without<MainCamera>)>,
) {
    if let Ok(main_tf) = main_camera.get_single() {
        if let Ok(mut reflect_tf) = reflection_camera.get_single_mut() {
            // Mirror position across Z=0
            let pos = main_tf.translation;
            reflect_tf.translation = Vec3::new(pos.x, pos.y, -pos.z);

            // Mirror rotation
            // We want the camera to look at the mirrored target.
            // Main camera look direction:
            let forward = main_tf.forward();
            // Reflected forward vector (flip Z)
            let reflected_forward = Vec3::new(forward.x, forward.y, -forward.z);

            // Calculate look target
            let target = reflect_tf.translation + reflected_forward;

            // Up vector should also be mirrored?
            // Main Up is usually +Y (in Bevy default) but we are looking down -Z?
            // Wait, Bevy Y is Up? No, in this game Z is up (Curling sheet on XY plane).
            // Main camera looks -Z (mostly).
            // Reflected camera looks +Z.
            // Up vector of main camera is likely +Y (along sheet).
            // Let's use look_at.

            // If main camera is at (0, -50, 2) looking at (0, 17, 0)
            // Reflected camera is at (0, -50, -2) looking at (0, 17, 0)

            // But wait, look_at relies on "Up".
            // If main Up is Z, reflected Up is -Z?
            // No, main Up is roughly Y?

            // Let's calculate the rotation manually or use look_at with appropriate Up.
            // If we just mirror the position and look at the mirrored focus point.

            // Focus point distance = some arbitrary value?
            // Or just use the forward vector.

            reflect_tf.look_to(Dir3::new(reflected_forward).unwrap_or(Dir3::Y), Vec3::Y);

            // Wait, if Z is up.
            // Main camera Up is likely Z? No, it's looking -Z.
            // Let's check setup.rs: looking_at(..., Vec3::Z)
            // So Up is Z.

            // If we reflect across Z=0, the "Up" vector (Z) becomes -Z.
            reflect_tf.look_to(Dir3::new(reflected_forward).unwrap_or(Dir3::Z), -Vec3::Z);
        }
    }
}
