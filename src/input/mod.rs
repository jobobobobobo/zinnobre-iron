use avian3d::{
    math::Scalar,
    prelude::{LinearVelocity, ShapeHits},
};
use bevy::{
    ecs::query::QueryData,
    prelude::{Component, Query, Transform},
    reflect::Reflect,
};
use leafwing_input_manager::input_processing::WithDualAxisProcessingPipelineExt;
use leafwing_input_manager::prelude::{
    ActionState, Actionlike, DualAxisProcessor, DualAxisSensitivity, InputMap, KeyboardVirtualDPad,
    MouseMove,
};
use lightyear::prelude::{Deserialize, Serialize};

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect, Serialize, Deserialize)]
pub enum BasicAction {
    #[actionlike(DualAxis)]
    Move,
    #[actionlike(DualAxis)]
    Look,
}

impl BasicAction {
    pub fn mkb_input_map() -> InputMap<Self> {
        InputMap::default()
            .with_dual_axis(
                Self::Look,
                MouseMove::default().with_processor(DualAxisProcessor::Sensitivity(
                    DualAxisSensitivity::all(0.03),
                )),
            )
            .with_dual_axis(Self::Move, KeyboardVirtualDPad::WASD)
    }
}

#[derive(QueryData)]
#[query_data(mutable, derive(Debug))]
pub(crate) struct MovementQuery {
    basic_mov: &'static mut BasicMovement,
    lin_vel: &'static mut LinearVelocity,
    transform: &'static Transform,
}

#[derive(QueryData)]
#[query_data(derive(Debug))]
pub(crate) struct GroundQuery {
    ground_hits: Option<&'static ShapeHits>,
    grounded: Option<&'static Grounded>,
}

//TODO: maybe I don't need dt here if I'm doing instant accel like overwatch?
pub(crate) fn sys_movement(
    // time: Res<Time>,
    action_state: &ActionState<BasicAction>,
    mut mov_query: Query<MovementQuery>,
) {
    // let dt = time.delta_seconds();

    let move_dir = action_state
        .axis_pair(&BasicAction::Move)
        .clamp_length_max(1.0);

    for mut mov in &mut mov_query {
        let vel = mov.basic_mov.move_speed;
        let wish_mov = move_dir * vel;
        mov.lin_vel.x = wish_mov.x;
        mov.lin_vel.z = wish_mov.y;
    }
}

#[derive(Component, Reflect, Debug)]
pub(crate) struct Grounded;

#[derive(Component, Reflect, Debug)]
pub(crate) struct BasicMovement {
    pub(crate) move_speed: Scalar,
    pub(crate) ground_tick: u8,
}

impl Default for BasicMovement {
    fn default() -> Self {
        Self {
            move_speed: 5.5,
            ground_tick: 0,
        }
    }
}
