use bevy::prelude::Reflect;
use leafwing_input_manager::input_processing::WithDualAxisProcessingPipelineExt;
use leafwing_input_manager::prelude::DualAxisProcessor;
use leafwing_input_manager::prelude::DualAxisSensitivity;
use leafwing_input_manager::prelude::InputMap;
use leafwing_input_manager::prelude::KeyboardVirtualDPad;
use leafwing_input_manager::prelude::MouseMove;
use leafwing_input_manager::Actionlike;
use lightyear::prelude::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect, Serialize, Deserialize)]
pub enum BasicAction {
    Move,
    Look,
}

impl Actionlike for BasicAction {
    fn input_control_kind(&self) -> leafwing_input_manager::InputControlKind {
        leafwing_input_manager::InputControlKind::DualAxis
    }
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
