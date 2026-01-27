use num_derive::FromPrimitive;

/// Digital buttons on a PlayStation controller. On ps1, the value assigned to each button is the bit
/// position in the 16bit word returned in the serial protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, FromPrimitive)]
pub enum Button {
    Select = 0,
    L3 = 1,
    R3 = 2,
    Start = 3,
    DUp = 4,
    DRight = 5,
    DDown = 6,
    DLeft = 7,
    L2 = 8,
    R2 = 9,
    L1 = 10,
    R1 = 11,
    Triangle = 12,
    Circle = 13,
    Cross = 14,
    Square = 15,
    Analog = 0xff,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ButtonState {
    Pressed,
    Released,
}

impl ButtonState {
    pub(crate) fn is_pressed(self) -> bool {
        self == ButtonState::Pressed
    }
}