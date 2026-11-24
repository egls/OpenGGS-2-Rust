use bevy::prelude::*;

const STICK_DEADZONE: f32 = 0.45;

pub fn any_gamepad_left_held(gamepads: &Query<&Gamepad>) -> bool {
    gamepads.iter().any(|gp| {
        gp.pressed(GamepadButton::DPadLeft)
            || gp.get(GamepadAxis::LeftStickX).unwrap_or(0.0) < -STICK_DEADZONE
    })
}

pub fn any_gamepad_right_held(gamepads: &Query<&Gamepad>) -> bool {
    gamepads.iter().any(|gp| {
        gp.pressed(GamepadButton::DPadRight)
            || gp.get(GamepadAxis::LeftStickX).unwrap_or(0.0) > STICK_DEADZONE
    })
}

pub fn any_gamepad_down_held(gamepads: &Query<&Gamepad>) -> bool {
    gamepads.iter().any(|gp| {
        gp.pressed(GamepadButton::DPadDown)
            || gp.get(GamepadAxis::LeftStickY).unwrap_or(0.0) < -STICK_DEADZONE
    })
}

pub fn any_gamepad_jump_held(gamepads: &Query<&Gamepad>) -> bool {
    gamepads.iter().any(|gp| {
        gp.pressed(GamepadButton::South)
            || gp.pressed(GamepadButton::DPadUp)
            || gp.get(GamepadAxis::LeftStickY).unwrap_or(0.0) > STICK_DEADZONE
    })
}

pub fn any_gamepad_just_jump(gamepads: &Query<&Gamepad>) -> bool {
    gamepads.iter().any(|gp| {
        gp.just_pressed(GamepadButton::South) || gp.just_pressed(GamepadButton::DPadUp)
    })
}

pub fn any_gamepad_just_shoot(gamepads: &Query<&Gamepad>) -> bool {
    gamepads.iter().any(|gp| {
        gp.just_pressed(GamepadButton::West)
            || gp.just_pressed(GamepadButton::East)
            || gp.just_pressed(GamepadButton::RightTrigger)
    })
}

pub fn any_gamepad_just_pause(gamepads: &Query<&Gamepad>) -> bool {
    gamepads.iter().any(|gp| {
        gp.just_pressed(GamepadButton::Start) || gp.just_pressed(GamepadButton::Select)
    })
}

pub fn any_gamepad_nav_up(gamepads: &Query<&Gamepad>) -> bool {
    gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::DPadUp))
}

pub fn any_gamepad_nav_down(gamepads: &Query<&Gamepad>) -> bool {
    gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::DPadDown))
}

pub fn any_gamepad_nav_left(gamepads: &Query<&Gamepad>) -> bool {
    gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::DPadLeft))
}

pub fn any_gamepad_nav_right(gamepads: &Query<&Gamepad>) -> bool {
    gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::DPadRight))
}

pub fn any_gamepad_confirm(gamepads: &Query<&Gamepad>) -> bool {
    gamepads.iter().any(|gp| {
        gp.just_pressed(GamepadButton::South) || gp.just_pressed(GamepadButton::Start)
    })
}

pub fn any_gamepad_cancel(gamepads: &Query<&Gamepad>) -> bool {
    gamepads.iter().any(|gp| {
        gp.just_pressed(GamepadButton::East) || gp.just_pressed(GamepadButton::Select)
    })
}
