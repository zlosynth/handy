use crate::control_input::ControlInputInterface;

pub fn warm_up_control_input(control_input_interface: &mut ControlInputInterface) {
    for _ in 0..100 {
        control_input_interface.sample();
    }
}
